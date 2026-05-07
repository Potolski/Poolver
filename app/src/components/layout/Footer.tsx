import { PoolverWordmark } from "@/components/brand/PoolverLogo";
import { POOLVER_CORE_PROGRAM_ID } from "@/lib/constants";
import { truncateAddress } from "@/lib/utils";

const SECTIONS = {
  proto: ["Whitepaper", "GitHub", "Audit Report", "Program ID", "Status"],
  poolver: ["Browse Pools", "Create Pool", "Reputation", "Treasury", "Reserve"],
  social: ["@poolver", "Discord", "Telegram EN", "Telegram PT", "Dialect"],
};

export function Footer() {
  const programIdShort = truncateAddress(POOLVER_CORE_PROGRAM_ID.toBase58(), 8);
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
${programIdShort}
IDL v1.0 · devnet
`}
          </pre>
        </div>
        <div>
          <h4>Protocol</h4>
          <ul>
            {SECTIONS.proto.map((x) => (
              <li key={x}>
                <a href="#">{x} ↗</a>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h4>Poolver</h4>
          <ul>
            {SECTIONS.poolver.map((x) => (
              <li key={x}>
                <a href="#">{x}</a>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h4>Social</h4>
          <ul>
            {SECTIONS.social.map((x) => (
              <li key={x}>
                <a href="#">{x}</a>
              </li>
            ))}
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
