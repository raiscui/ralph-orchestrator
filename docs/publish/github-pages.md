# GitHub Pages Publishing

This site is a static MkDocs Material build published through GitHub Pages.
The workflow builds `site/`, uploads it as a Pages artifact, then deploys that
artifact through the official Pages action.

## Why This Stack

MkDocs Material is already the repository-supported documentation stack:

- `mkdocs.yml` exists at the workspace root
- README badges point to a MkDocs documentation site
- `.github/workflows/docs.yml` already builds and deploys docs
- GitHub Pages accepts arbitrary static artifacts from Actions

Using this existing stack avoids adding a second JavaScript documentation
framework just to publish static Markdown.

## Local Build

```bash
uvx --from mkdocs --with mkdocs-material --with mkdocs-minify-plugin --with mkdocs-material-extensions --with pymdown-extensions mkdocs build --strict
```

The generated output is `site/`, which is ignored by git.

## Workflow Shape

The deploy workflow should keep these jobs separate:

1. build docs
2. upload the `site/` artifact
3. deploy the artifact to the `github-pages` environment

This matches GitHub Pages' custom workflow model and keeps pull requests as
build-only validation while pushes to `main` or `master` deploy.

## GitHub Pages Settings

For this workflow to publish, the repository Pages source must be configured for
GitHub Actions in repository settings. The workflow itself handles the artifact
and deployment, but repository-level Pages settings still control whether that
deployment source is accepted.

## External References

- GitHub Pages custom workflows: <https://docs.github.com/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages>
- MkDocs deployment guide: <https://www.mkdocs.org/user-guide/deploying-your-docs/>
- Material for MkDocs publishing guide: <https://squidfunk.github.io/mkdocs-material/publishing-your-site/>
