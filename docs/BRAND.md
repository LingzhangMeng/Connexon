# Connexon brand guide

![Connexon mark](../assets/logo-128.png)

## The mark

The Connexon mark is a hexagonal **connexon** — the half-channel that two cells
contribute to a gap junction. Two halves meet around a central channel.

| Element | Meaning |
|---|---|
| Hexagon | The hexameric ring of a connexon |
| Left teal half | R and its statistical ecosystem |
| Right amber half | Python and its model ecosystem |
| Ivory channel | Connexon, the runtime that joins them |
| Navy field | Rust, the core beneath both |
| Navy pore | The lumen through which data crosses |

The mark is deliberately simple: it stays legible at 24 px and needs no text to be
recognised.

## Assets

| File | Size | Use |
|---|---|---|
| `assets/logo.svg` | vector | Primary mark, navy rounded square |
| `assets/logo.png` | 1024 × 1024 | GitHub avatar, documentation, social preview source |
| `assets/logo-512.png` | 512 × 512 | Smaller raster uses |
| `assets/logo-128.png` | 128 × 128 | Small raster uses |
| `assets/logo-transparent.svg` | vector | Transparent mark for light backgrounds |
| `assets/logo-transparent.png` | 1024 × 1024 | Transparent raster |
| `assets/favicon.svg` | vector | Browser tab icon |
| `assets/social-preview.png` | 1280 × 640 | GitHub social preview |

## Colour palette

| Name | Hex | Role |
|---|---|---|
| Navy | `#0B1B2B` | Background, core, pore |
| Teal | `#2A9D8F` | R half, accents |
| Amber | `#E9A23B` | Python half, highlights |
| Ivory | `#F7F3E9` | Channel, primary text on navy |
| Muted blue | `#9FB3C8` | Secondary text on navy |

Contrast notes:

- Ivory on navy is high contrast and suitable for body text.
- Teal and amber carry the two ecosystems; do not swap them.
- Do not place the ivory channel on a light background — use the transparent variant,
  which has a navy channel.

## Clear space

Keep at least **one quarter of the mark's width** free on every side. Do not place text,
another logo or a border inside that space.

## Minimum size

| Medium | Minimum |
|---|---|
| Digital | 24 px |
| Print | 10 mm |
| Favicon | 16–32 px (use `favicon.svg`) |

## Backgrounds

| Background | Use |
|---|---|
| Light | `logo-transparent.svg` / `logo-transparent.png` |
| Dark | `logo.svg` / `logo.png` (the navy square provides contrast) |
| Busy photograph | `logo.png` on the navy rounded square only |
| GitHub avatar | `logo.png` |
| GitHub social preview | `social-preview.png` |

## Do

- Use the supplied files.
- Preserve the aspect ratio.
- Keep the teal half on the left and the amber half on the right.
- Use the primary mark on dark backgrounds and the transparent mark on light ones.
- Use `social-preview.png` for the repository's social preview.

## Do not

- Recolour, rotate, stretch, skew or crop the mark.
- Separate the two halves or remove the channel.
- Add shadows, gradients, outlines or other effects.
- Change the channel to a low-contrast colour.
- Use the transparent variant on a dark background.
- Use the mark to imply endorsement by the R, Python or Rust projects.

## Typography

| Use | Recommended | Fallback |
|---|---|---|
| Headings and wordmark | Inter, IBM Plex Sans | system sans-serif |
| Code | JetBrains Mono, IBM Plex Mono | Consolas, monospace |

The supplied raster assets use a geometric sans; the repository does not bundle font
files. Do not redraw the wordmark in a different typeface.

## Usage

### README

```markdown
![Connexon logo](assets/logo-128.png)
```

### GitHub avatar

Upload `assets/logo.png` under **Settings → General → Social preview** or the
organisation avatar settings.

### GitHub social preview

Upload `assets/social-preview.png` (1280 × 640) under **Settings → General → Social
preview**.

### Favicon

Reference `assets/favicon.svg`:

```html
<link rel="icon" href="assets/favicon.svg" type="image/svg+xml">
```

## Licence

The Connexon mark is part of this repository and is released under the MIT licence.
No trademark claim is made over the word “Connexon” or the mark.
