import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { getAccount } from "@solana/spl-token";
import { PoolverClient, findProtocolFeeVault, findReserveVault, microUsdcToHuman } from "../client/src";

(async () => {
  const conn = new Connection("https://devnet.helius-rpc.com/?api-key=222244e4-11ac-447d-b17e-b0397ca5ca49", "confirmed");
  const provider = new AnchorProvider(conn, new Wallet(Keypair.generate()), { commitment: "confirmed" });
  const client = new PoolverClient(provider);
  const accounts = await (client.core.account as any).pool.all();

  let activePools = 0, completedPools = 0, formingPools = 0;
  let totalContributedMicro = new BN(0);
  let totalDistributedMicro = new BN(0);
  let totalCollateralMicro = new BN(0);
  for (const { account } of accounts) {
    if (account.isComplete) completedPools++;
    else if (account.currentMonth === 0) formingPools++;
    else activePools++;
    totalContributedMicro = totalContributedMicro.add(account.totalContributed);
    totalDistributedMicro = totalDistributedMicro.add(account.totalDistributed);
    totalCollateralMicro = totalCollateralMicro.add(account.totalCollateralLocked);
  }

  const [feeVault] = findProtocolFeeVault();
  const [vaultRes] = findReserveVault("vault");
  const [defiRes] = findReserveVault("defi");
  const fee = Number(microUsdcToHuman(new BN((await getAccount(conn, feeVault)).amount.toString())));
  const vRes = Number(microUsdcToHuman(new BN((await getAccount(conn, vaultRes)).amount.toString())));
  const dRes = Number(microUsdcToHuman(new BN((await getAccount(conn, defiRes)).amount.toString())));

  const fmt = (n: number) => `$${n >= 1000 ? (n/1000).toFixed(1) + "K" : n.toFixed(2)}`;
  console.log("\n══ DEVNET SNAPSHOT ══\n");
  console.log(`POOLS:               ${accounts.length} total · ${completedPools} completed · ${activePools} active · ${formingPools} forming`);
  console.log(`TOTAL CONTRIBUTIONS: ${fmt(Number(microUsdcToHuman(totalContributedMicro)))}`);
  console.log(`TOTAL DISTRIBUTED:   ${fmt(Number(microUsdcToHuman(totalDistributedMicro)))}`);
  console.log(`COLLATERAL LOCKED:   ${fmt(Number(microUsdcToHuman(totalCollateralMicro)))}`);
  console.log(`\nPROTOCOL FEES:       ${fmt(fee)}`);
  console.log(`VAULT-TIER RESERVE:  ${fmt(vRes)}`);
  console.log(`DEFI-TIER RESERVE:   ${fmt(dRes)}`);
})();
