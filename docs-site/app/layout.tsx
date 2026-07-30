import type { Metadata } from "next";
import { headers } from "next/headers";
import "./globals.css";

export async function generateMetadata(): Promise<Metadata> {
  const requestHeaders = await headers();
  const host = requestHeaders.get("x-forwarded-host") ?? requestHeaders.get("host") ?? "localhost:3000";
  const protocol = requestHeaders.get("x-forwarded-proto") ?? (host.startsWith("localhost") ? "http" : "https");
  const metadataBase = new URL(`${protocol}://${host}`);

  return {
    metadataBase,
    title: {
      default: "Zeta 工程文档",
      template: "%s · Zeta 文档",
    },
    description: "Zeta 的架构、实现契约、运行路径与演进边界。",
    openGraph: {
      title: "Zeta 工程文档",
      description: "架构 · 契约 · 运行路径",
      images: [{ url: "/og.png", width: 1731, height: 909, alt: "Zeta 工程文档" }],
      type: "website",
    },
    twitter: {
      card: "summary_large_image",
      title: "Zeta 工程文档",
      description: "架构 · 契约 · 运行路径",
      images: ["/og.png"],
    },
  };
}

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  );
}
