#!/usr/bin/env npx tsx
/**
 * One-shot script to:
 *   1. Call admin_close_protocol on poolver-core
 *   2. Call admin_close_reserve(Vault) on poolver-reserve
 *   3. Call admin_close_reserve(DeFi) on poolver-reserve
 *   4. Re-run initialize_protocol with the new mint
 *   5. Re-run initialize_reserve(Vault) and (DeFi)
 *
 * Usage:
 *   NODE_PATH=./client/node_modules npx tsx scripts/admin-close-and-reinit.ts \
 *     --rpc "https://devnet.helius-rpc.com/?api-key=..." \
 *     --new-mint B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8 \
 *     --wallet ./deploy-keypair.json
 */
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import { AnchorProvider, BN, Wallet, Program } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { readFileSync } from "fs";
import { resolve } from "path";

import {
  PoolverClient,
  POOLVER_CORE_PROGRAM_ID,
  POOLVER_RESERVE_PROGRAM_ID,
  PROTOCOL_CONFIG_SEED,
  PROTOCOL_FEE_VAULT_SEED,
  findProtocolConfig,
} from "../client/src";

function findReserveFundDirect(tier: number): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("reserve_fund"), Buffer.from([tier])],
    POOLVER_RESERVE_PROGRAM_ID
  );
}

interface Args {
  rpc: string;
  newMint: PublicKey;
  walletPath: string;
}

function parseArgs(argv: string[]): Args {
  const get = (k: string): string | undefined => {
    const i = argv.indexOf(`--${k}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  return {
    rpc: get("rpc") ?? "https://api.devnet.solana.com",
    newMint: new PublicKey(get("new-mint") ?? "B6dnuZtKH7FsSK6tySfWkk6ReW2LdKpmnfGAoMKsv8w8"),
    walletPath: resolve(get("wallet") ?? "./deploy-keypair.json"),
  };
}

function loadKeypair(path: string): Keypair {
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path, "utf8"))));
}

function findReserveVault(programId: PublicKey, tier: number): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("reserve_vault"), Buffer.from([tier])],
    programId
  );
}

function findProtocolFeeVault(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [PROTOCOL_FEE_VAULT_SEED],
    POOLVER_CORE_PROGRAM_ID
  );
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const admin = loadKeypair(args.walletPath);
  const conn = new Connection(args.rpc, "confirmed");
  const wallet = new Wallet(admin);
  const provider = new AnchorProvider(conn, wallet, { commitment: "confirmed" });

  const client = new PoolverClient({ connection: conn, wallet, cluster: "devnet" });

  console.log(`[admin] signer=${admin.publicKey.toBase58()} new-mint=${args.newMint.toBase58()}`);

  // 1. admin_close_protocol
  const [protocolConfig] = findProtocolConfig();
  const [protocolFeeVault] = findProtocolFeeVault();
  console.log(`[1/6] admin_close_protocol — config=${protocolConfig.toBase58()} feeVault=${protocolFeeVault.toBase58()}`);
  try {
    const sig = await client.core.methods
      .adminCloseProtocol()
      .accounts({
        admin: admin.publicKey,
        protocolConfig,
        protocolFeeVault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([admin])
      .rpc();
    console.log(`  → ${sig}`);
  } catch (e: any) {
    if (e.message?.includes("not initialized") || e.message?.includes("does not exist") || e.message?.includes("AccountNotInitialized")) {
      console.log(`  → already closed (skip)`);
    } else {
      throw e;
    }
  }

  // 2. admin_close_reserve(Vault)
  for (const [tierName, tierByte] of [["vault", 0], ["deFi", 1]] as const) {
    const [reserveFund] = findReserveFundDirect(tierByte);
    const [reserveVault] = findReserveVault(POOLVER_RESERVE_PROGRAM_ID, tierByte);
    const stepNum = tierByte === 0 ? "2/6" : "3/6";
    console.log(`[${stepNum}] admin_close_reserve(${tierName}) — fund=${reserveFund.toBase58()} vault=${reserveVault.toBase58()}`);
    const tierIdl = tierByte === 0 ? { vault: {} } : { deFi: {} };
    try {
      const sig = await client.reserve.methods
        .adminCloseReserve(tierIdl)
        .accounts({
          caller: admin.publicKey,
          reserveFund,
          reserveUsdcVault: reserveVault,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([admin])
        .rpc();
      console.log(`  → ${sig}`);
    } catch (e: any) {
      if (e.message?.includes("not initialized") || e.message?.includes("does not exist") || e.message?.includes("AccountNotInitialized")) {
        console.log(`  → already closed (skip)`);
      } else {
        throw e;
      }
    }
  }

  // 4-6. Re-run initialize for protocol + both reserves
  console.log(`[4/6] initialize_protocol with new mint`);
  const initIxAccounts = client.core.idl
    ? undefined
    : undefined;

  // Use existing initialize.ts logic inline
  const [pCfg] = findProtocolConfig();
  const [pFee] = findProtocolFeeVault();
  try {
    const sig = await client.core.methods
      .initializeProtocol()
      .accounts({
        deployer: admin.publicKey,
        protocolConfig: pCfg,
        usdcMint: args.newMint,
        protocolFeeVault: pFee,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: new PublicKey("SysvarRent111111111111111111111111111111111"),
      })
      .signers([admin])
      .rpc();
    console.log(`  → ${sig}`);
  } catch (e: any) {
    if (e.message?.includes("already in use") || e.message?.includes("already initialized")) {
      console.log(`  → already initialized`);
    } else {
      throw e;
    }
  }

  for (const [tierName, tierByte] of [["vault", 0], ["deFi", 1]] as const) {
    const stepNum = tierByte === 0 ? "5/6" : "6/6";
    console.log(`[${stepNum}] initialize_reserve(${tierName})`);
    const [rFund] = findReserveFundDirect(tierByte);
    const [rVault] = findReserveVault(POOLVER_RESERVE_PROGRAM_ID, tierByte);
    const tierIdl = tierByte === 0 ? { vault: {} } : { deFi: {} };
    try {
      const sig = await client.reserve.methods
        .initializeReserve(tierIdl)
        .accounts({
          admin: admin.publicKey,
          reserveFund: rFund,
          reserveUsdcVault: rVault,
          usdcMint: args.newMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: new PublicKey("SysvarRent111111111111111111111111111111111"),
        })
        .signers([admin])
        .rpc();
      console.log(`  → ${sig}`);
    } catch (e: any) {
      if (e.message?.includes("already in use") || e.message?.includes("already initialized")) {
        console.log(`  → already initialized`);
      } else {
        throw e;
      }
    }
  }

  console.log(`[done] all rotation steps complete.`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
