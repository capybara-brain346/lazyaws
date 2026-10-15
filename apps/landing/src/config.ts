export const site = {
  name: "Lazy AWS",
  description:
    "Your AWS resources. One terminal. Browse, inspect, and run confirmed AWS CLI actions without leaving your keyboard.",
  github: "https://github.com/capybara-brain346/lazyaws",
  author: "https://piyushc.com",
  rust: "https://www.rust-lang.org/tools/install",
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
    title: "Get the source",
    description: "Clone the repository and open the project.",
    command: `git clone ${site.github}.git\ncd lazyaws`,
    label: "Clone repository",
  },
  {
    title: "Take it for a spin",
    description:
      "Start with local sample data. No AWS account or credentials needed.",
    command: "cargo run -- --demo",
    label: "Run demo",
  },
  {
    title: "Connect your account",
    description:
      "Use an existing AWS CLI profile. Sign in first if your profile uses SSO.",
    command:
      "aws sso login --profile dev\ncargo run -- --profile dev --region eu-west-1",
    label: "Connect AWS profile",
  },
  {
    title: "Make yourself at home",
    description: "Build the release binary and see all available options.",
    command: "cargo build --release\n./target/release/lazyaws --help",
    label: "Build release",
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
