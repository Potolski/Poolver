"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { PoolverMark } from "@/components/brand/PoolverLogo";
import { WalletButton } from "@/components/wallet/WalletButton";
import { ThemeToggle } from "@/components/layout/ThemeToggle";

const NAV = [
  { href: "/", key: "landing", label: "Home" },
  { href: "/pools", key: "pools", label: "Pools" },
  { href: "/create", key: "create", label: "Create" },
  { href: "/docs", key: "docs", label: "Docs" },
] as const;

export function TopBar() {
  const pathname = usePathname();
  const [menuOpen, setMenuOpen] = useState(false);

  function isActive(href: string, key: string): boolean {
    if (key === "landing") return pathname === "/";
    if (key === "pools") return pathname === "/pools" || pathname.startsWith("/pool");
    return pathname.startsWith(href);
  }

  useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!menuOpen) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setMenuOpen(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menuOpen]);

  return (
    <header className="topbar">
      <Link href="/" className="brand">
        <PoolverMark size={22} />
        <b>Poolver</b>
        <span className="dim2 topbar-tag">v1.0</span>
      </Link>
      <nav className="topbar-nav-desktop">
        {NAV.map((item) => (
          <Link
            key={item.key}
            href={item.href}
            className={cn(isActive(item.href, item.key) && "on")}
          >
            {item.label}
          </Link>
        ))}
      </nav>
      <div className="right">
        <span className="pill live topbar-pill-desktop">
          <span className="dot" />
          Solana · Devnet
        </span>
        <ThemeToggle />
        <WalletButton />
        <button
          type="button"
          className="topbar-burger"
          aria-label={menuOpen ? "Close menu" : "Open menu"}
          aria-expanded={menuOpen}
          aria-controls="mobile-nav-panel"
          onClick={() => setMenuOpen((v) => !v)}
        >
          <span className={cn("burger-icon", menuOpen && "open")}>
            <span />
            <span />
            <span />
          </span>
        </button>
      </div>

      <div
        id="mobile-nav-panel"
        className={cn("mobile-nav-panel", menuOpen && "open")}
        role="dialog"
        aria-modal="true"
        aria-hidden={!menuOpen}
      >
        <nav className="mobile-nav">
          {NAV.map((item) => (
            <Link
              key={item.key}
              href={item.href}
              className={cn("mobile-nav-link", isActive(item.href, item.key) && "on")}
            >
              <span className="mobile-nav-num">
                {String(NAV.indexOf(item) + 1).padStart(2, "0")}
              </span>
              <span className="mobile-nav-label">{item.label}</span>
              <span className="mobile-nav-arrow">→</span>
            </Link>
          ))}
        </nav>
        <div className="mobile-nav-foot">
          <span className="pill live">
            <span className="dot" />
            Solana · Devnet
          </span>
        </div>
      </div>
      {menuOpen && (
        <button
          type="button"
          className="mobile-nav-scrim"
          aria-label="Close menu"
          onClick={() => setMenuOpen(false)}
        />
      )}
    </header>
  );
}
