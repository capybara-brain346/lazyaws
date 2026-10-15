"use client";

import { useEffect, useRef, useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { site } from "@/config";

export function CopyButton({
  command,
  label,
}: {
  command: string;
  label: string;
}) {
  const [status, setStatus] = useState<"idle" | "copied" | "error">("idle");
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setStatus("copied");
    } catch {
      setStatus("error");
    }

    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setStatus("idle"), site.copyFeedbackMs);
  }

  return (
    <span className="copy-control">
      <Button
        variant="ghost"
        size="icon"
        onClick={copy}
        aria-label={
          status === "copied"
            ? `${label} copied`
            : `Copy ${label.toLowerCase()}`
        }
      >
        {status === "copied" ? <Check /> : <Copy />}
      </Button>
      <span
        className={status === "error" ? "copy-error" : "sr-only"}
        role="status"
      >
        {status === "copied"
          ? "Copied to clipboard."
          : status === "error"
            ? "Copy failed. Select the command to copy it."
            : ""}
      </span>
    </span>
  );
}
