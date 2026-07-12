/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  basePath: "/ui",
  reactStrictMode: true,
  images: { unoptimized: true },
};

export default nextConfig;
