import Image from "next/image";
import { ArrowUpRight } from "lucide-react";
import { site, tour } from "@/config";

export function QuickTour() {
  return (
    <section
      id="tour"
      className="tour-section section-shell"
      aria-labelledby="tour-title"
    >
      <div className="tour-heading">
        <div>
          <h2 id="tour-title">
            A small learning curve.
            <br />
            <span>A lot less clicking.</span>
          </h2>
          <p>Three stops. Then you know your way around.</p>
        </div>
        <Image
          src={site.tourIcon}
          width={128}
          height={128}
          alt=""
          className="section-icon"
        />
      </div>
      <ol className="tour-timeline">
        {tour.map((step, index) => (
          <li key={step.title}>
            <div className="tour-copy">
              <span className="tour-node" aria-hidden="true">
                {index + 1}
              </span>
              <Image
                src={step.icon}
                width={96}
                height={96}
                alt=""
                className="tour-icon"
              />
              <h3>{step.title}</h3>
              <p>{step.description}</p>
            </div>
            <a
              className="tour-image-link"
              href={step.image}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={`Open full-size image: ${step.title}`}
            >
              <Image
                src={step.image}
                alt={step.alt}
                width={1040}
                height={578}
              />
              <span>
                View demo capture <ArrowUpRight size={14} />
              </span>
            </a>
          </li>
        ))}
      </ol>
      <div className="tour-end">
        <span className="end-node" />
        <p>
          That’s the short version.{" "}
          <a
            href={`${site.github}/blob/main/docs/services.md`}
            target="_blank"
            rel="noopener noreferrer"
          >
            Explore the service guide <ArrowUpRight size={14} />
          </a>
        </p>
      </div>
    </section>
  );
}
