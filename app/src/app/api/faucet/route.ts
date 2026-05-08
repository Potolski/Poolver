import "server-only";

import { NextResponse } from "next/server";
import {
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { USDC_MINT_DEVNET_DEFAULT } from "@poolver/client";

import { getAdminClient } from "@/lib/admin-client";
import { getClientIp, rateLimit, scrubError } from "@/lib/rate-limit";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface FaucetBody {
  recipient?: string;
  amount?: number;
}

const HARD_CAP = 100_000;
const DEFAULT_AMOUNT = 100_000;
const MICRO_PER_USDC = BigInt(1_000_000);

export async function POST(req: Request) {
  let body: FaucetBody;
  try {
    body = (await req.json()) as FaucetBody;
  } catch {
    return NextResponse.json({ error: "invalid_json" }, { status: 400 });
  }
  if (!body.recipient || typeof body.recipient !== "string") {
    return NextResponse.json({ error: "recipient_required" }, { status: 400 });
  }
  let recipient: PublicKey;
  try {
    recipient = new PublicKey(body.recipient);
  } catch {
    return NextResponse.json({ error: "invalid_recipient" }, { status: 400 });
  }
  const amount = body.amount && Number.isFinite(body.amount) ? body.amount : DEFAULT_AMOUNT;
  if (amount <= 0 || amount > HARD_CAP) {
    return NextResponse.json(
      { error: "amount_out_of_range", maxAmount: HARD_CAP },
      { status: 400 }
    );
  }

  // Demo-friendly rate limits: 30 req/min per IP, 6 req/min per recipient.
  // Tight production limits live in scripts/faucet.ts comments and should
  // come back when this exits demo mode.
  const ip = getClientIp(req);
  if (!rateLimit("ip", ip, 30, 60_000)) {
    return NextResponse.json({ error: "rate_limited" }, { status: 429 });
  }
  if (!rateLimit("recipient", recipient.toBase58(), 6, 60_000)) {
    return NextResponse.json({ error: "rate_limited" }, { status: 429 });
  }

  let connection;
  let keypair;
  try {
    ({ connection, keypair } = getAdminClient());
  } catch (e) {
    return NextResponse.json(
      {
        error: "admin_unavailable",
        message: scrubError(e instanceof Error ? e.message : String(e)),
      },
      { status: 503 }
    );
  }

  const mint = USDC_MINT_DEVNET_DEFAULT;
  const recipientAta = getAssociatedTokenAddressSync(mint, recipient);
  const amountMicro = BigInt(Math.floor(amount)) * MICRO_PER_USDC;

  try {
    const tx = new Transaction()
      .add(
        createAssociatedTokenAccountIdempotentInstruction(
          keypair.publicKey,
          recipientAta,
          recipient,
          mint
        )
      )
      .add(
        createMintToInstruction(
          mint,
          recipientAta,
          keypair.publicKey,
          amountMicro
        )
      );
    const sig = await sendAndConfirmTransaction(connection, tx, [keypair], {
      commitment: "confirmed",
    });
    return NextResponse.json({
      signature: sig,
      recipientAta: recipientAta.toBase58(),
      amount,
      mint: mint.toBase58(),
    });
  } catch (e) {
    const raw = e instanceof Error ? e.message : String(e);
    return NextResponse.json(
      { error: "faucet_failed", message: scrubError(raw) },
      { status: 500 }
    );
  }
}
