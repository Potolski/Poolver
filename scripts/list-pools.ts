import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { PoolverClient } from "../client/src";

(async () => {
  const conn = new Connection("https://devnet.helius-rpc.com/?api-key=222244e4-11ac-447d-b17e-b0397ca5ca49", "confirmed");
  const provider = new AnchorProvider(conn, new Wallet(Keypair.generate()), { commitment: "confirmed" });
  const client = new PoolverClient(provider);
  const accounts = await (client.core.account as any).pool.all();
  for (const { publicKey, account } of accounts) {
    // raw.tier shape
    const tier = JSON.stringify(account.tier);
    console.log(`${publicKey.toBase58()}  tier=${tier}  current_month=${account.currentMonth}  is_complete=${account.isComplete}`);
  }
})();
