import Link from "next/link";
import Image from "next/image";
import { site } from "@/config";

export function Brand() {
  return (
    <Link href="#top" className="brand" aria-label="Lazy AWS home">
      <Image
        src={site.logo}
        width={384}
        height={213}
        alt=""
        className="brand-mark"
      />
      <span>
        lazy<span className="brand-aws">aws</span>
        <span className="brand-period">.</span>
      </span>
    </Link>
  );
}
