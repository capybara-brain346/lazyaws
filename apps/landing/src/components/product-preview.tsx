"use client";

import Image from "next/image";
import { useEffect, useState } from "react";
import { Pause, Play, Terminal } from "lucide-react";
import { services, serviceIcons, site } from "@/config";

export function ProductPreview() {
  const [service, setService] = useState<(typeof services)[number]>(
    services[0],
  );
  const [isPlaying, setIsPlaying] = useState(true);
  const image = `/images/demo-${service.toLowerCase()}.webp`;

  useEffect(() => {
    if (!isPlaying) return;

    const timer = window.setTimeout(() => {
      setService(
        (current) =>
          services[(services.indexOf(current) + 1) % services.length],
      );
    }, site.previewIntervalMs);

    return () => window.clearTimeout(timer);
  }, [service, isPlaying]);

  return (
    <div className="product-preview">
      <div className="window-bar">
        <span className="window-dots" aria-hidden="true">
          <i />
          <i />
          <i />
        </span>
        <span>
          <Terminal size={13} /> lazyaws — demo
        </span>
      </div>
      <div className="service-tabs" aria-label="Preview an AWS service">
        {services.map((name) => (
          <button
            type="button"
            key={name}
            aria-label={name}
            title={name}
            aria-pressed={service === name}
            onClick={() => setService(name)}
          >
            <Image src={serviceIcons[name]} width={26} height={26} alt="" />
          </button>
        ))}
        <button
          type="button"
          className="preview-toggle"
          aria-label={
            isPlaying ? "Pause service preview" : "Resume service preview"
          }
          title={isPlaying ? "Pause service preview" : "Resume service preview"}
          onClick={() => setIsPlaying((current) => !current)}
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} />}
        </button>
      </div>
      <Image
        src={image}
        alt={`Real Lazy AWS ${service} demo. Resource lists appear beside the selected resource’s JSON details.`}
        width={1040}
        height={578}
        priority
        className="terminal-image"
      />
      <div className="preview-caption">
        <span>Real interface. Sample resources.</span>
        <span>Made for your keyboard.</span>
      </div>
    </div>
  );
}
