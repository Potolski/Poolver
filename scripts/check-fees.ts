import { Connection, PublicKey } from "@solana/web3.js";
import { getAccount } from "@solana/spl-token";
import { findProtocolFeeVault, findReserveVault, microUsdcToHuman } from "../client/src";

const RPC = "https://devnet.helius-rpc.com/?api-key=222244e4-11ac-447d-b17e-b0397ca5ca49";

(async () => {
  const conn = new Connection(RPC, "confirmed");
  const [protocolFeeVault] = findProtocolFeeVault();
  const [vaultReserveUsdc] = findReserveVault("vault");
  const [defiReserveUsdc] = findReserveVault("defi");

  console.log("\n══ FEE / RESERVE ADDRESSES ══\n");
  console.log(`Protocol fee vault       (1.5% of every contribution + winning bid)`);
  console.log(`  ${protocolFeeVault.toBase58()}`);
  console.log(`  https://explorer.solana.com/address/${protocolFeeVault.toBase58()}?cluster=devnet`);
  console.log(`\nVault reserve USDC vault (1.5% from Vault-tier pools)`);
  console.log(`  ${vaultReserveUsdc.toBase58()}`);
  console.log(`  https://explorer.solana.com/address/${vaultReserveUsdc.toBase58()}?cluster=devnet`);
  console.log(`\nDeFi reserve USDC vault  (2.5% from DeFi-tier pools)`);
  console.log(`  ${defiReserveUsdc.toBase58()}`);
  console.log(`  https://explorer.solana.com/address/${defiReserveUsdc.toBase58()}?cluster=devnet`);

  console.log("\n══ LIVE BALANCES ══\n");
  for (const [label, pk] of [
    ["protocol_fee_vault   ", protocolFeeVault],
    ["vault_reserve.usdc   ", vaultReserveUsdc],
    ["defi_reserve.usdc    ", defiReserveUsdc],
  ] as const) {
    try {
      const acct = await getAccount(conn, pk);
      const human = Number(microUsdcToHuman(BigInt(acct.amount.toString()) as any));
      console.log(`${label}: $${human.toLocaleString()} USDC  (raw: ${acct.amount})`);
    } catch (e: any) {
      console.log(`${label}: <missing>  (${e.message?.slice(0, 60)})`);
    }
  }
})();
