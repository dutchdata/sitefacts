# sitefacts

One facts file → every machine-readable file a small website needs. Deterministic, dependency-free output,
and a `--check` that fails CI when a file is stale.

```sh
cargo install sitefacts
sitefacts init > facts.json        # an example to edit
sitefacts build                    # writes the stale outputs under site/
sitefacts build --check            # exit 1 if any output is stale (CI, pre-commit, deploy scripts)
sitefacts show llms.txt            # print one output instead of writing it
```

Inputs: `facts.json` and `site/index.html` (its `<title>`, its body text, and a block between
`<!-- machine:begin` and `<!-- machine:end -->` in the head that gets rewritten). Outputs, all under `site/`:

| file | what |
|---|---|
| `index.html` (head block only) | meta description, robots, canonical, discovery links, `rel="me"` for every profile, Open Graph, Twitter card, inline JSON-LD |
| `organization.jsonld` | the same schema.org graph (Organization, WebSite, WebPage, ImageObject), one fetch |
| `llms.txt` | the llmstxt.org summary: description, the home page's text verbatim, facts, pages, profiles, retrieval notes |
| `llms-full.txt` | everything on the site in one file, plus policies and the graph |
| `ai.txt` | usage policy for AI systems, with an optional attribution-parameter request |
| `humans.txt` | who runs the site and how it is built |
| `robots.txt` | RFC 9309, 24 AI and search crawlers named, `Content-Signal`, sitemap |
| `sitemap.xml` | every page with `lastmod`, image extension for page images |
| `.well-known/security.txt` | RFC 9116, when `security_email` is set |
| `site.webmanifest` + `manifest.json` | identical; the second is for probes |
| `<indexnow_key>.txt` | the IndexNow key file, when `indexnow_key` is set |

Why generate them: the same facts (profiles, contact, description, dates) appear in up to six files each.
Hand-kept copies drift, and a retrieval system handed two versions of a fact has no way to pick one. Edit the
facts, never the outputs.

## The facts file

Required: `updated` (YYYY-MM-DD), `url` (canonical, with trailing slash), `legal_name`, `brand`, `description`,
`email`, `logo` and `og_image` (`path`, `width`, `height`, `caption`/`alt`). Image sizes are checked against the
files' headers (PNG, WebP), so facts and files cannot disagree.

Optional, each adding its section or field when present: `kind` ("research company", "studio"; default
"company"), `org_type` (schema.org type, default `Organization`), `principles` (one sentence; becomes
`publishingPrinciples` and the "one ask" in ai.txt), `knows_about` (with Wikidata ids), `security_email` +
`security_txt_expires`, `duns`, `profiles`, `twitter_site`, `icons`, `page_images`, `pages` (path, title,
summary, image, updated), `indexnow_key`, `url_note`, `one_line`, `pages_intro`, `retrieval_notes`, `ai_notes`,
`attribution` (`{name, values}`), `privacy`, `humans` (`does`, `built_with`, `analytics`, `hosted_on`,
`standards`, `pages`), `manifest` (`display`, `theme_color`, `background_color`), `nav_class` (block elements
with this class are navigation, not page text; default `apps`). Unknown keys are ignored, so the file can carry
your own notes (`_doc`).

## Guarantees

- Same facts and page → same bytes, on any machine. `--check` compares bytes.
- Text is wrapped at 78 columns exactly as Python's `textwrap` does (the files were born there; diffs stay small).
- JSON-LD is written once and used twice (inline and standalone), with `</` escaped in the inline copy.
- No network. Nothing outside `site/` is touched.

## Where it came from

Built for [dutchdata.io](https://www.dutchdata.io/), whose machine-readable layer it generates. The Python
script it replaced produced byte-identical files for every output except two intentional fixes (the page count
in humans.txt was stale; two hand-wrapped paragraphs are now wrapped by the tool).

MIT or Apache-2.0.
