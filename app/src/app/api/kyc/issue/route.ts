import "server-only";

import { NextResponse } from "next/server";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
  findKycAttestation,
  mockIssueKycIx,
  type KycLevelName,
} from "@poolver/client";

import { getAdminClient } from "@/lib/admin-client";
import { getClientIp, rateLimit, scrubError } from "@/lib/rate-limit";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface IssueBody {
  user?: string;
  level?: KycLevelName;
}

export async function POST(req: Request) {
  let body: IssueBody;
  try {
    body = (await req.json()) as IssueBody;
  } catch {
    return NextResponse.json({ error: "invalid_json" }, { status: 400 });
  }
  if (!body.user || typeof body.user !== "string") {
    return NextResponse.json({ error: "user_required" }, { status: 400 });
  }
  let user: PublicKey;
  try {
    user = new PublicKey(body.user);
  } catch {
    return NextResponse.json({ error: "invalid_user_pubkey" }, { status: 400 });
  }
  const level: KycLevelName = body.level ?? "full";
  if (level !== "light" && level !== "full") {
    return NextResponse.json({ error: "invalid_level" }, { status: 400 });
  }

  // Demo-friendly rate limits: 30 req/min per IP, 6 req/min per recipient.
  // Tighten before exposing to public traffic.
  const ip = getClientIp(req);
  if (!rateLimit("ip", ip, 30, 60_000)) {
    return NextResponse.json({ error: "rate_limited" }, { status: 429 });
  }
  if (!rateLimit("recipient", user.toBase58(), 6, 60_000)) {
    return NextResponse.json({ error: "rate_limited" }, { status: 429 });
  }

  let client;
  try {
    ({ client } = getAdminClient());
  } catch (e) {
    return NextResponse.json(
      {
        error: "admin_unavailable",
        message: scrubError(e instanceof Error ? e.message : String(e)),
      },
      { status: 503 }
    );
  }

  // Detect mainnet IDL (mock_issue_kyc absent on production builds).
  const idlIxs = (client.core.idl as { instructions?: { name: string }[] })
    .instructions;
  if (idlIxs && !idlIxs.some((ix) => ix.name === "mockIssueKyc" || ix.name === "mock_issue_kyc")) {
    return NextResponse.json(
      {
        error: "mainnet_build",
        message:
          "mock_issue_kyc not in IDL — production KYC integration pending.",
      },
      { status: 503 }
    );
  }

  const [kycPda] = findKycAttestation(user);

  try {
    const ix = await mockIssueKycIx(client, { user, level });
    const tx = new Transaction().add(ix);
    const sig = await client.provider.sendAndConfirm!(tx, [], {
      commitment: "confirmed",
    });
    return NextResponse.json({
      signature: sig,
      kycPda: kycPda.toBase58(),
      level,
    });
  } catch (e) {
    const raw = e instanceof Error ? e.message : String(e);
    if (
      raw.includes("already in use") ||
      raw.includes("already exists") ||
      raw.includes("0x0") // anchor "AccountAlreadyInUse"
    ) {
      return NextResponse.json({
        idempotent: true,
        kycPda: kycPda.toBase58(),
        level,
      });
    }
    return NextResponse.json(
      { error: "kyc_issue_failed", message: scrubError(raw) },
      { status: 500 }
    );
  }
}
