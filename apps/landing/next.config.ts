import type { NextConfig } from "next";
import path from "node:path";

const config: NextConfig = {
  output: "export",
  turbopack: { root: path.join(__dirname, "../..") },
  devIndicators: false,
  images: { unoptimized: true },
};

export default config;
