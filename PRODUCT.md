# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Next.js in `apps/landing`, with shadcn/ui and Lucide icons. The user selected a code-first build.

## Product Purpose

Lazy AWS is a terminal interface for AWS. This website helps users understand it, run the local demo, and learn the main controls.

## Users

Developers and operators who use the AWS CLI. This audience follows from the repository requirements.

## Capabilities and Constraints

Browse EC2, ECS, ECR, S3, and Lambda. Inspect JSON and confirm AWS CLI actions. Demo mode uses local sample data. Installation uses Cargo commands; package distribution is not ready.

## Brand Commitments

Use the supplied sleeping-cloud logo from `assets/icons/lazyaws.png`. Use the supplied AWS service icons in the web demo controls.

One product. One page. Logo and GitHub link in the header. No sign-up. Railway is the main visual reference. The user approved a dark infrastructure style with real demo captures. Use generated hero artwork. End with a short interface guide and a footer that links “Built by Piyush Chaudhary” to https://piyushc.com.

## Evidence on Hand

README.md, docs/services.md, docs/demo.txt, src/demo.rs, and 33 reference captures in screenshots/. The source remote is https://github.com/capybara-brain346/lazyaws.

## Product Principles

- Show the actual interface.
- Make the demo easy to run.
- Keep commands and key bindings accurate.
- Make no unsupported performance or adoption claims.
