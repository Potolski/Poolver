import { PublicKey, Transaction, VersionedTransaction } from "@solana/web3.js";
import type { Provider as AppKitSolanaProvider } from "@reown/appkit-adapter-solana/react";

export interface AnchorWalletShape {
  publicKey: PublicKey;
  signTransaction<T extends Transaction | VersionedTransaction>(tx: T): Promise<T>;
  signAllTransactions<T extends Transaction | VersionedTransaction>(txs: T[]): Promise<T[]>;
}

/**
 * Build an Anchor-compatible wallet shim from the Reown AppKit Solana
 * provider. The duck-typed shape (publicKey + sign* methods) is what
 * AnchorProvider actually uses at runtime; the formal `Wallet` class is
 * Node-only and not importable in browser bundles.
 */
export function appKitToAnchorWallet(
  walletProvider: AppKitSolanaProvider,
  publicKey: PublicKey
): AnchorWalletShape {
  const signTransaction = walletProvider.signTransaction.bind(walletProvider);
  const signAllTransactions = walletProvider.signAllTransactions
    ? walletProvider.signAllTransactions.bind(walletProvider)
    : async <T extends Transaction | VersionedTransaction>(txs: T[]): Promise<T[]> => {
        const out: T[] = [];
        for (const tx of txs) out.push(await signTransaction(tx));
        return out;
      };

  return {
    publicKey,
    signTransaction: signTransaction as AnchorWalletShape["signTransaction"],
    signAllTransactions: signAllTransactions as AnchorWalletShape["signAllTransactions"],
  };
}
