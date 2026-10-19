import Image from "next/image";
import { ArrowDown, ArrowRight, ArrowUpRight } from "lucide-react";
import { GithubIcon as Github } from "@/components/github-icon";
import { Brand } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { ProductPreview } from "@/components/product-preview";
import { InstallGuide } from "@/components/install-guide";
import { QuickTour } from "@/components/quick-tour";
import { site } from "@/config";

export default function Home() {
  return (
    <div id="top">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="site-header">
        <nav className="header-inner" aria-label="Main navigation">
          <Brand />
          <a
            className="github-link"
            href={site.github}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Github size={17} />
            <span>Open source on GitHub</span>
            <ArrowUpRight size={14} />
          </a>
        </nav>
      </header>
      <main id="main">
        <section className="hero" aria-labelledby="hero-title">
          <div className="hero-copy">
            <a
              className="hero-badge"
              href={site.productHunt.href}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Image
                src={site.productHunt.badge}
                alt={site.productHunt.alt}
                width={site.productHunt.width}
                height={site.productHunt.height}
              />
            </a>
            <h1 id="hero-title">
              Your cloud.
              <br />
              <span>Your terminal.</span>
            </h1>
            <p>
              Meet Lazy AWS. A keyboard-driven home for your AWS resources.
              <br className="desktop-break" /> Browse, inspect, and act. Without
              the console shuffle.
            </p>
            <div className="hero-actions">
              <Button asChild>
                <a href="#install">
                  Get started <ArrowRight size={16} />
                </a>
              </Button>
              <a className="quiet-link" href="#tour">
                Take a quick tour <ArrowDown size={14} />
              </a>
            </div>
          </div>
          <div className="hero-stage">
            <Image
              src="/images/valley.webp"
              alt=""
              fill
              priority
              sizes="100vw"
              className="landscape"
            />
            <div className="landscape-shade" />
            <div className="preview-wrap">
              <ProductPreview />
            </div>
          </div>
        </section>
        <InstallGuide />
        <QuickTour />
      </main>
      <footer className="site-footer">
        <div className="footer-inner">
          <Brand />
          <div className="footer-badges">
            <a
              className="footer-badge"
              href={site.usefulShelf.href}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Image
                src={site.usefulShelf.badge}
                alt={site.usefulShelf.alt}
                width={site.usefulShelf.width}
                height={site.usefulShelf.height}
              />
            </a>
          </div>
          <a href={site.author} target="_blank" rel="noopener noreferrer">
            Built by <span>Piyush Choudhari</span>
            <ArrowUpRight size={14} />
          </a>
        </div>
      </footer>
    </div>
  );
}
