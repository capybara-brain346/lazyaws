import distribution from "../../../distribution/config.json";

export const site = {
  name: "Lazy AWS",
  description:
    "Your AWS resources. One terminal. Browse, inspect, and run confirmed AWS CLI actions without leaving your keyboard.",
  homepage: distribution.homepage,
  github: `https://github.com/${distribution.repository}`,
  author: "https://piyushc.com",
  productHunt: {
    href: "https://www.producthunt.com/products/lazy-aws-your-cloud-your-terminal?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-lazy-aws-your-cloud-your-terminal",
    badge:
      "https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1282153&theme=light&t=1792199437708",
    alt: "Lazy AWS — Your cloud. Your terminal. - Meet Lazy AWS. A keyboard-driven home for your AWS resources | Product Hunt",
    width: 250,
    height: 54,
  },
  usefulShelf: {
    href: "https://usefulshelf.co/?utm_source=lazyaws.xyz&utm_medium=referral&utm_campaign=badge&utm_content=lime",
    badge: "https://usefulshelf.co/badge/usefulshelf.svg?theme=lime",
    alt: "Featured on UsefulShelf",
    width: 248,
    height: 66,
  },
  support:
    "We support macOS, Ubuntu, Arch Linux, and Omarchy. Windows is not supported.",
  installNote:
    "Choose curl or Homebrew to install on macOS or Linux. Then run the demo or connect your AWS account. Debian installs are available without a support guarantee. AWS CLI v2 and credentials are needed only for your AWS account.",
  awsCli:
    "https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html",
  copyFeedbackMs: 2500,
  previewIntervalMs: 2000,
  logo: "/icons/lazyaws-mark.webp",
  installIcon: "/icons/install.webp",
  tourIcon: "/icons/tour.webp",
  icons: [
    { url: "/favicon.svg", type: "image/svg+xml", sizes: "any" },
    {
      url: "/favicon.ico",
      type: "image/x-icon",
      sizes: "16x16 32x32 48x48 64x64 128x128 256x256",
    },
  ],
};

export const installation = [
  {
    title: "Install with curl",
    description: `Install the binary to ~/${distribution.installDir}. Add this directory to your PATH if needed.`,
    command: `curl -fsSL ${site.homepage}/install.sh | sh`,
    label: "Install with curl",
  },
  {
    title: "Or install with Homebrew",
    description: "Install from our tap on macOS or Linux with Homebrew.",
    command: `brew install ${distribution.tap}/${distribution.name}`,
    label: "Install with Homebrew",
  },
  {
    title: "Run the installed demo",
    description:
      "Start with local sample data. No AWS account or credentials needed.",
    command: "lazyaws --demo",
    label: "Run demo",
  },
  {
    title: "Connect your account",
    description:
      "Use an existing AWS CLI profile. Sign in first if your profile uses SSO.",
    command:
      "aws sso login --profile dev\nlazyaws --profile dev --region eu-west-1",
    label: "Connect AWS profile",
  },
] as const;

export const services = ["EC2", "ECS", "ECR", "S3", "Lambda"] as const;

export const serviceIcons = {
  EC2: "/icons/ec2.svg",
  ECS: "/icons/ecs.svg",
  ECR: "/icons/ecr.svg",
  S3: "/icons/s3.svg",
  Lambda: "/icons/lambda.svg",
} as const;

export const tour = [
  {
    title: "Find your bearings.",
    description:
      "Switch services, move between panes, and select a resource. Your account and region stay in view.",
    icon: "/icons/browse.webp",
    image: "/images/demo-ec2.webp",
    alt: "Lazy AWS EC2 demo with instance, volume, security group, and subnet panes beside JSON details.",
  },
  {
    title: "Go one level deeper.",
    description:
      "Filter a list, then open the selected resource’s JSON details. Related resources are right beside it.",
    icon: "/icons/inspect.webp",
    image: "/images/demo-details.webp",
    alt: "Lazy AWS demo with the EC2 JSON detail pane focused.",
  },
  {
    title: "Know what will run.",
    description:
      "Open an action and review the account, target, and exact AWS CLI command. Run it only when you confirm.",
    icon: "/icons/confirm.webp",
    image: "/images/demo-action.webp",
    alt: "Lazy AWS demo action confirmation showing the AWS CLI command before execution.",
  },
] as const;
