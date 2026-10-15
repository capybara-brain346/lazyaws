import type { NextConfig } from "next";

const config: NextConfig = {
  output: "export",
  devIndicators: false,
  images: { unoptimized: true },
};

export default config;
