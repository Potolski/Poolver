"use client";

import { TopBar } from "./TopBar";
import { Footer } from "./Footer";
import { WalletRedirect } from "@/components/wallet/WalletRedirect";
import { OnboardingGate } from "@/components/onboarding/OnboardingGate";

export function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <>
      <TopBar />
      <WalletRedirect />
      <main>{children}</main>
      <OnboardingGate />
      <Footer />
    </>
  );
}
