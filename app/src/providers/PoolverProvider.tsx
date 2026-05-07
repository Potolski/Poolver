"use client";

import { createContext, useContext, useMemo } from "react";
import { useAppKitConnection } from "@reown/appkit-adapter-solana/react";
import { useAppKitProvider, useAppKitAccount } from "@reown/appkit/react";
import type { Provider } from "@reown/appkit-adapter-solana/react";
import {
  Connection,
  PublicKey,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";
import { PoolverClient, type PoolverClientOpts } from "@poolver/client";

import { appKitToAnchorWallet, type AnchorWalletShape } from "@/lib/wallet-bridge";
import { DEVNET_RPC } from "@/lib/constants";

interface PoolverContextType {
  client: PoolverClient;
  connected: boolean;
  address: string | undefined;
  publicKey: PublicKey | null;
}

const READ_ONLY_PUBKEY = PublicKey.default;

function makeReadOnlyWallet(): AnchorWalletShape {
  return {
    publicKey: READ_ONLY_PUBKEY,
    async signTransaction<T extends Transaction | VersionedTransaction>(_tx: T): Promise<T> {
      throw new Error("read-only client cannot sign");
    },
    async signAllTransactions<T extends Transaction | VersionedTransaction>(_txs: T[]): Promise<T[]> {
      throw new Error("read-only client cannot sign");
    },
  };
}

function buildReadOnlyClient(connection: Connection): PoolverClient {
  return new PoolverClient({
    connection,
    wallet: makeReadOnlyWallet() as unknown as PoolverClientOpts["wallet"],
  });
}

const PoolverContext = createContext<PoolverContextType | null>(null);

export function PoolverProvider({ children }: { children: React.ReactNode }) {
  const { connection } = useAppKitConnection();
  const { walletProvider } = useAppKitProvider<Provider>("solana");
  const { address, isConnected } = useAppKitAccount();

  const publicKey = useMemo(() => {
    if (!address) return null;
    try {
      return new PublicKey(address);
    } catch {
      return null;
    }
  }, [address]);

  const client = useMemo<PoolverClient>(() => {
    const conn = connection ?? new Connection(DEVNET_RPC, "confirmed");
    if (!isConnected || !publicKey || !walletProvider) {
      return buildReadOnlyClient(conn);
    }
    try {
      const wallet = appKitToAnchorWallet(walletProvider, publicKey);
      return new PoolverClient({
        connection: conn,
        wallet: wallet as unknown as PoolverClientOpts["wallet"],
      });
    } catch {
      return buildReadOnlyClient(conn);
    }
  }, [connection, walletProvider, publicKey, isConnected]);

  return (
    <PoolverContext.Provider
      value={{
        client,
        connected: isConnected,
        address,
        publicKey,
      }}
    >
      {children}
    </PoolverContext.Provider>
  );
}

export function usePoolver(): PoolverContextType {
  const ctx = useContext(PoolverContext);
  if (!ctx) {
    throw new Error("usePoolver must be used inside <PoolverProvider>");
  }
  return ctx;
}
