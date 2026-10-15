---
name: Lazy AWS
description: A calm home for a terminal tool.
colors:
  background: "#0b0e14"
  surface: "#111620"
  surface-raised: "#171e2a"
  foreground: "#edf1f7"
  muted: "#a4aebe"
  soft: "#c2cad8"
  accent: "#efb98d"
  accent-hover: "#f7cba7"
  border: "#29313f"
  success: "#91b8a0"
  primary-ink: "#20170f"
typography:
  display:
    fontFamily: '"Manrope Variable", sans-serif'
    fontSize: "clamp(56px, 6.2vw, 84px)"
    fontWeight: 550
    lineHeight: 1.08
    letterSpacing: "-0.04em"
  headline:
    fontFamily: '"Manrope Variable", sans-serif'
    fontSize: "36px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "-0.035em"
  title:
    fontFamily: '"Manrope Variable", sans-serif'
    fontSize: "22px"
    fontWeight: 500
    lineHeight: 1.8
    letterSpacing: "-0.015em"
  body:
    fontFamily: '"Manrope Variable", sans-serif'
    fontSize: "14px"
    fontWeight: 400
  code:
    fontFamily: '"JetBrains Mono Variable", monospace'
    fontSize: "12px"
    lineHeight: 2
rounded:
  control: "6px"
  code: "7px"
  capture: "8px"
  preview: "12px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.primary-ink}"
    rounded: "{rounded.control}"
    padding: "0 20px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.control}"
    size: "36px"
  code-block:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.code}"
---

<!-- Source: apps/landing/src/app/{base,hero,guides,responsive}.css, imported by globals.css. -->

# Design System: Lazy AWS

## Overview

**Creative North Star: "A terminal in a night landscape"**

Dark navy surfaces and a quiet landscape frame the real terminal. Pale text supports long reading. Apricot marks actions and selected controls. The landscape adds depth; actual demo captures show the product.

The page has open space around headings and compact detail inside terminal and command panels. Local fonts keep the same type across visits.

**Key Characteristics:**
- Dark navy surfaces with pale text.
- Apricot actions and selected controls.
- Real terminal captures with full-size links.
- Open section spacing and compact command panels.

## Colors

### Primary

Apricot (`accent`) marks the main action, selection, focus, and timeline nodes. The lighter apricot (`accent-hover`) shows primary button hover. Dark ink (`primary-ink`) keeps text clear on that button.

### Secondary

The top preview has no local data status indicator.

### Neutral

Night navy (`background`) is the page base. `surface` holds commands; `surface-raised` shows ghost button hover. `foreground` is the main text. `soft` supports secondary links and lead copy; `muted` supports descriptions. `border` separates sections and panels.

**The Action Color Rule.** Use apricot for actions, selection, and focus. Use neutral colors for large text areas and panel fills.

## Typography

Manrope Variable is the display and body font, with a sans-serif fallback. JetBrains Mono Variable is the code and terminal-label font, with a monospace fallback. Both fonts are local assets.

The frontmatter records the desktop hierarchy. Main headings use medium weight and tight letter spacing. Tour titles use the title role. Small installation titles use 14px at weight 600. Descriptions use 13–15px with generous line height. Code uses the mono role; terminal labels use smaller mono text.

**The Code Type Rule.** Use the mono face for commands, step numbers, and terminal labels. Use Manrope for explanatory text.

## Layout

Shared section content has a maximum width of 1120px. The terminal preview has a maximum width of 840px, set by `--preview-width`. Installation and tour rows use two columns on wide screens. The tour joins numbered nodes with a vertical line and short curved branches.

```text
Wide section:    [ Explanation ] [ Commands or capture ]
Narrow section:  [ Explanation         ]
                 [ Commands or capture ]
```

At 1200px and below, side margins are 48px. At 760px and below, side margins become 24px and the installation and tour use one column. The main heading then uses `clamp(44px, 13.2vw, 60px)`; section headings become 31px. At 380px and below, the main heading uses `clamp(42px, 13.2vw, 50px)`.

Scrollbars are hidden on the page and command panels. Normal page scrolling and horizontal command scrolling remain enabled. Captures keep their aspect ratio and have links to the full-size image. The footer can wrap on small screens.

## Elevation & Depth

Surface tones and thin borders separate most content. The main terminal has the only large shadow (`0 26px 70px rgb(0 0 0 / 45%)`). A dark gradient joins the landscape to the page background.

The terminal enters once, over one second, with a small upward move and a brightness change. Controls use 0.2-second transitions. Reduced-motion settings remove transitions and animation and use immediate anchor scrolling.

## Shapes

Panels have small, soft corners. Buttons use the control radius; command panels use the code radius; capture links use the capture radius. The large terminal uses the preview radius. Timeline nodes, status dots, and window controls are circles. Thin borders give structure without heavy outlines.

## Components

### Buttons

The primary button has an apricot fill, dark text, a minimum height of 43px, and a small arrow. Hover lightens the fill and moves it up by 1px. Ghost icon buttons are 36px square; hover adds the raised surface fill and brighter text. Keyboard focus has a 2px apricot outline with a 5px offset.

### Navigation

The header pairs the supplied sleeping-cloud artwork and wordmark with the GitHub link. The footer repeats the wordmark. Quiet links use soft text and become apricot on hover.

### Terminal Preview

A compact window bar and service buttons with the supplied AWS SVG icons sit above real demo images. Buttons retain service names as accessible labels and hover titles. The selected service has a thin apricot bottom border. The preview advances through all five services every two seconds and repeats. Selecting a service changes the capture and restarts the timer. A pause/resume button controls automatic changes. The top preview has no full-size link or local sample data indicator. Its window title remains centered at each viewport.

### Command Panels

Commands sit in a bordered surface with a small label and a ghost copy button. Copy success changes the icon and announces the result. Copy failure shows text that tells the user to select the command.

### Section Artwork

The installation section, guide heading, and three guide steps use generated cloud illustrations. Each matches the supplied logo’s navy body, gold light, and soft shape. Use 128px artwork at section level and 96px at step level. On phones, the guide heading uses 72px and steps use 80px. The hero has no feature-label row. The guide has no shortcut badges.

```text
Supplied cloud logo
  +--> terminal cloud: installation
  +--> route cloud: guide
  +--> compass / lens / shield: guide steps
```

### Tour Captures

Capture links have a thin border, rounded corners, and a compact link label below the image. Hover makes the label apricot. A numbered branch connects each capture to its short explanation.

## Do's and Don'ts

### Do:
- **Do** use real demo captures to explain the terminal.
- **Do** keep commands in the mono face.
- **Do** provide visible keyboard focus and full-size capture links.
- **Do** keep the reduced-motion behavior.

### Don't:
- **Don't** use apricot as a large panel fill.
- **Don't** add the large terminal shadow to every panel.
- **Don't** stretch terminal captures or place them in narrow fixed-width boxes.
