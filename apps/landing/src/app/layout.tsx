import type { Metadata } from "next";
import "@fontsource-variable/manrope";
import "@fontsource-variable/jetbrains-mono";
import "./globals.css";
import { site } from "@/config";

export const metadata: Metadata = {
  metadataBase: new URL(site.homepage),
  title: "Lazy AWS — Your cloud. Your terminal.",
  description: site.description,
  icons: { icon: site.icons },
  openGraph: {
    title: "Lazy AWS — Your cloud. Your terminal.",
    description: site.description,
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
