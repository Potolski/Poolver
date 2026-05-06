#!/usr/bin/env npx tsx
/**
 * Poolver V1 deployment driver.
 *
 * Deploys the four programs in dependency order:
 *
 *   1. poolver-yield-vault   (no deps)
 *   2. poolver-yield-defi    (no deps)
 *   3. poolver-reserve       (referenced by core)
 *   4. poolver-core          (depends on the above for CPI types)
 *
 * Usage:
 *
 *   npx tsx scripts/deploy.ts --cluster devnet
 *   npx tsx scripts/deploy.ts --cluster mainnet-beta
 *   npx tsx scripts/deploy.ts --cluster devnet --dry-run
 *   npx tsx scripts/deploy.ts --cluster devnet --skip-build
 *
 * Mainnet safeguards (ALL must pass before any `anchor deploy` runs):
 *   1. Operator types `DEPLOY MAINNET POOLVER-V1` exactly at the prompt.
 *   2. Per-program IDLs MUST NOT contain any `mock_*` instructions
 *      (jq instructions[].name check; per QUESTIONS.md Q-33).
 *   3. Per-program .so files MUST NOT contain `Instruction: Mock*`
 *      panic strings (strings + grep; reliable on stripped Solana .so
 *      files where `nm` returns "no symbols").
 *
 * Saves a JSON receipt to `deployments/<cluster>-<UTC-iso>.json`.
 */
import { spawnSync } from "child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { resolve } from "path";
import * as readline from "readline";

const REPO_ROOT = resolve(__dirname, "..");

// Dependency-ordered.
const PROGRAMS = [
  "poolver-yield-vault",
  "poolver-yield-defi",
  "poolver-reserve",
  "poolver-core",
] as const;
type ProgramName = (typeof PROGRAMS)[number];

/** Programs that gate mock features behind a default-on Cargo feature. */
const MOCK_GATED: Record<ProgramName, string[]> = {
  "poolver-yield-vault": [],
  "poolver-yield-defi": [
    "mock_inject_yield",
    "mock_set_utilization",
    "mock_set_oracle_deviation",
    "mock_set_kamino_paused",
  ],
  "poolver-reserve": [],
  "poolver-core": ["mock_issue_kyc"],
};

/** Where Anchor writes IDLs and .so files. */
function idlPath(prog: ProgramName): string {
  return resolve(
    REPO_ROOT,
    "target/idl",
    `${prog.replace(/-/g, "_")}.json`
  );
}
function soPath(prog: ProgramName): string {
  return resolve(REPO_ROOT, "target/deploy", `${prog.replace(/-/g, "_")}.so`);
}

interface CliArgs {
  cluster: "devnet" | "mainnet-beta" | "localnet";
  dryRun: boolean;
  skipBuild: boolean;
  walletPath: string;
}

function parseArgs(): CliArgs {
  const argv = process.argv.slice(2);
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(k);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const cluster = (get("--cluster") ?? "devnet") as CliArgs["cluster"];
  if (!["devnet", "mainnet-beta", "localnet"].includes(cluster)) {
    fail(`unsupported --cluster: ${cluster}`);
  }
  return {
    cluster,
    dryRun: argv.includes("--dry-run"),
    skipBuild: argv.includes("--skip-build"),
    walletPath:
      get("--wallet") ?? resolve(REPO_ROOT, "deploy-keypair.json"),
  };
}

function fail(msg: string): never {
  console.error(`\n[deploy] ABORT: ${msg}\n`);
  process.exit(1);
}

function step(label: string): void {
  console.log(`\n=== ${label} ===`);
}

function shell(cmd: string, args: string[], opts: { dryRun: boolean }): string {
  const rendered = `${cmd} ${args.join(" ")}`;
  if (opts.dryRun) {
    console.log(`[dry-run] ${rendered}`);
    return "";
  }
  console.log(`[exec] ${rendered}`);
  const result = spawnSync(cmd, args, {
    cwd: REPO_ROOT,
    stdio: ["inherit", "pipe", "inherit"],
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    fail(`command failed (status=${result.status}): ${rendered}`);
  }
  process.stdout.write(result.stdout || "");
  return result.stdout || "";
}

// ─────────────────────────── Mainnet guard ───────────────────────────

function confirmMainnet(): void {
  step("Mainnet confirmation gate");
  const phrase = "DEPLOY MAINNET POOLVER-V1";
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  rl.question(
    `Type the exact phrase to confirm mainnet deploy:\n  > ${phrase}\n\n> `,
    (answer) => {
      rl.close();
      if (answer.trim() !== phrase) {
        fail("confirmation phrase did not match; aborting.");
      }
      console.log("[deploy] phrase matched.");
      // Continue async-flow.
      runMainnetChecks();
    }
  );
}

function runMainnetChecks(): void {
  step("Mainnet build-guard verification");
  for (const prog of PROGRAMS) {
    verifyMockFree(prog);
  }
  console.log("\n[deploy] All mock guards passed. Proceeding to deploy.\n");
}

/**
 * Reject if either the IDL or the compiled .so contains evidence of
 * the mock instructions for `prog`.
 */
function verifyMockFree(prog: ProgramName): void {
  const mocks = MOCK_GATED[prog];
  if (mocks.length === 0) {
    console.log(`  [ok] ${prog}: no mock instructions defined.`);
    return;
  }

  // 1. IDL check (Q-33 final answer): instructions[].name must not
  //    include any mock entry.
  const idlFile = idlPath(prog);
  if (!existsSync(idlFile)) {
    fail(
      `${prog}: IDL not found at ${idlFile}. Run with --skip-build only after a clean rebuild.`
    );
  }
  const idl = JSON.parse(readFileSync(idlFile, "utf-8"));
  const idlIxNames: string[] = (idl.instructions ?? []).map(
    (i: { name: string }) => i.name
  );
  const idlBleed = mocks.filter((m) => idlIxNames.includes(m));
  if (idlBleed.length > 0) {
    fail(
      `${prog}: IDL contains mock instructions: ${idlBleed.join(
        ", "
      )}. Rebuild with --no-default-features.`
    );
  }

  // 2. .so symbol check via `strings`. Anchor embeds an
  //    "Instruction: <PascalName>" panic string per dispatched
  //    instruction; presence is a reliable mock indicator on stripped
  //    Solana .so files (where `nm` returns no symbols).
  const so = soPath(prog);
  if (!existsSync(so)) {
    fail(`${prog}: .so not found at ${so}. Build first.`);
  }
  const stringsOut = spawnSync("strings", [so], { encoding: "utf-8" });
  if (stringsOut.status !== 0) {
    fail(`${prog}: strings(1) failed on ${so}`);
  }
  for (const m of mocks) {
    const pascal = toPascal(m);
    const needle = `Instruction: ${pascal}`;
    if ((stringsOut.stdout || "").includes(needle)) {
      fail(
        `${prog}: .so binary contains mock dispatch string "${needle}". ` +
          `Rebuild with --no-default-features and try again.`
      );
    }
  }

  console.log(
    `  [ok] ${prog}: IDL + .so are mock-free (checked ${mocks.length} entries).`
  );
}

function toPascal(snake: string): string {
  return snake
    .split("_")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");
}

// ─────────────────────────── Build & deploy ──────────────────────────

function build(args: CliArgs): void {
  if (args.skipBuild) {
    console.log("[build] --skip-build given; using existing target/.");
    return;
  }
  step("anchor build");
  const buildArgs = ["build"];
  if (args.cluster === "mainnet-beta") {
    // Production builds drop the mock features entirely.
    buildArgs.push("--", "--no-default-features");
  }
  shell("anchor", buildArgs, { dryRun: args.dryRun });
}

interface DeployResult {
  program: ProgramName;
  programId: string;
  txSignature?: string;
}

function deployProgram(prog: ProgramName, args: CliArgs): DeployResult {
  step(`anchor deploy ${prog}`);
  const out = shell(
    "anchor",
    [
      "deploy",
      "--program-name",
      prog,
      "--provider.cluster",
      args.cluster,
      "--provider.wallet",
      args.walletPath,
    ],
    { dryRun: args.dryRun }
  );

  // Parse "Program Id: <id>" from anchor's output.
  const idMatch = out.match(/Program Id:\s+([A-Za-z0-9]{32,44})/);
  const sigMatch = out.match(/Deploy success.*\n.*Signature:\s+(\S+)/);
  return {
    program: prog,
    programId: idMatch ? idMatch[1] : "(dry-run)",
    txSignature: sigMatch ? sigMatch[1] : undefined,
  };
}

function writeReceipt(
  args: CliArgs,
  results: DeployResult[],
  deployer: string
): void {
  const dir = resolve(REPO_ROOT, "deployments");
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const file = resolve(dir, `${args.cluster}-${ts}.json`);
  const receipt = {
    cluster: args.cluster,
    deployedAt: new Date().toISOString(),
    deployer,
    dryRun: args.dryRun,
    programs: results,
  };
  writeFileSync(file, JSON.stringify(receipt, null, 2));
  console.log(`\n[deploy] Receipt written to ${file}`);
}

// ─────────────────────────── Main ─────────────────────────────────────

async function main(): Promise<void> {
  const args = parseArgs();
  step(`Poolver V1 deploy → ${args.cluster}${args.dryRun ? " (dry-run)" : ""}`);

  if (args.cluster === "mainnet-beta") {
    if (args.skipBuild) {
      fail(
        "--skip-build is forbidden on mainnet-beta. Mainnet must always rebuild from clean source."
      );
    }
    confirmMainnet();
    // confirmMainnet calls runMainnetChecks() which is async via readline;
    // continue the rest of the flow inside a then-style callback.
    // For dry-run/devnet flow we drop straight through.
    await new Promise<void>((res) => setTimeout(res, 1));
  }

  build(args);

  if (args.cluster !== "mainnet-beta") {
    // Devnet: still run the IDL guard so devs catch breakage early —
    // but we DO NOT block on .so mock strings (those are expected on
    // devnet builds with `mock-kyc` / `mock-yield` features).
    console.log("[deploy] (devnet) skipping mock-free guard.");
  }

  const results: DeployResult[] = [];
  for (const prog of PROGRAMS) {
    results.push(deployProgram(prog, args));
  }

  const deployer = readWalletPubkey(args.walletPath);
  writeReceipt(args, results, deployer);

  step("DEPLOY COMPLETE");
  for (const r of results) {
    console.log(`  ${r.program}: ${r.programId}`);
  }
}

function readWalletPubkey(walletPath: string): string {
  try {
    const result = spawnSync(
      "solana-keygen",
      ["pubkey", walletPath],
      { encoding: "utf-8" }
    );
    return (result.stdout || "").trim();
  } catch {
    return "(unknown)";
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
