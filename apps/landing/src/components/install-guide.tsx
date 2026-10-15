import Image from "next/image";
import { ArrowUpRight, Terminal } from "lucide-react";
import { installation, site } from "@/config";
import { CopyButton } from "@/components/copy-button";

export function InstallGuide() {
  return (
    <section
      id="install"
      className="install-section section-shell"
      aria-labelledby="install-title"
    >
      <div className="section-intro">
        <Image
          src={site.installIcon}
          width={128}
          height={128}
          alt=""
          className="section-icon"
        />
        <h2 id="install-title">
          Less setup.
          <br />
          <span>More getting things done.</span>
        </h2>
        <p>
          A terminal, Rust, and you. Start with the demo, then bring your own
          AWS account.
        </p>
        <div className="requirements">
          <span>Before you start</span>
          <a href={site.rust} target="_blank" rel="noopener noreferrer">
            Rust toolchain <ArrowUpRight />
          </a>
          <a href={site.awsCli} target="_blank" rel="noopener noreferrer">
            AWS CLI v2 <ArrowUpRight />
          </a>
        </div>
        <p className="install-note">
          Rust and Git are needed for the demo. AWS CLI v2 and configured
          credentials are needed for your account.
        </p>
      </div>
      <ol className="install-steps">
        {installation.map((step, index) => (
          <li key={step.title}>
            <span className="step-count">{index + 1}</span>
            <div className="install-step-content">
              <h3>{step.title}</h3>
              <p>{step.description}</p>
              <div className="code-block">
                <div className="code-label">
                  <Terminal size={13} />
                  <span>terminal</span>
                  <CopyButton command={step.command} label={step.label} />
                </div>
                <pre>
                  <code>{step.command}</code>
                </pre>
              </div>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
