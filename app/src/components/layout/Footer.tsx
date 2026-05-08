import Link from "next/link";

import { PoolverWordmark } from "@/components/brand/PoolverLogo";
import { POOLVER_CORE_PROGRAM_ID } from "@/lib/constants";
import { truncateAddress } from "@/lib/utils";

const PROGRAM_ID = POOLVER_CORE_PROGRAM_ID.toBase58();
const PROGRAM_ID_SHORT = truncateAddress(PROGRAM_ID, 8);

const GITHUB_URL = "https://github.com/Potolski/Poolver";
const X_URL = "https://x.com/poolverfi";
const EXPLORER_URL = `https://explorer.solana.com/address/${PROGRAM_ID}?cluster=devnet`;

function XIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}

export function Footer() {
  return (
    <footer className="shell footer">
      <div className="footer-grid">
        <div>
          <PoolverWordmark size={16} />
          <p className="footer-about">
            Poolver is a decentralized rotating savings + credit (consórcio /
            ROSCA) protocol on Solana. Bringing the $500B ROSCA market on-chain
            — no administrator, no geography, no permission.
          </p>
          <pre className="ascii footer-ascii">
{`POOLVER_CORE
${PROGRAM_ID_SHORT}
IDL v1.0 · devnet
`}
          </pre>
        </div>
        <div>
          <h4>Protocol</h4>
          <ul>
            <li>
              <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">
                GitHub ↗
              </a>
            </li>
            <li>
              <a
                href={EXPLORER_URL}
                target="_blank"
                rel="noopener noreferrer"
                title={PROGRAM_ID}
              >
                Program ID · {PROGRAM_ID_SHORT} ↗
              </a>
            </li>
          </ul>
        </div>
        <div>
          <h4>
            <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
              Poolver
              <a
                href={X_URL}
                target="_blank"
                rel="noopener noreferrer"
                aria-label="Poolver on X"
                title="@poolverfi on X"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  color: "var(--fg-3)",
                }}
              >
                <XIcon size={11} />
              </a>
            </span>
          </h4>
          <ul>
            <li>
              <Link href="/pools">Browse pools</Link>
            </li>
            <li>
              <Link href="/create">Create pool</Link>
            </li>
            <li>
              <Link href="/docs">Docs</Link>
            </li>
            <li>
              <Link href="/docs#08">Reputation system</Link>
            </li>
          </ul>
        </div>
      </div>
      <div className="footer-bot">
        <span>◆ Built on Solana · Switchboard VRF · SPL USDC</span>
        <span>Not an offer of securities · DYOR · © 2026</span>
      </div>
    </footer>
  );
}
