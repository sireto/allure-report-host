# Allure Report Host

A self-hosted service for uploading, storing, and serving [Allure 3](https://allurereport.org/) test reports from any CI pipeline. Built with Rust ([Axum](https://github.com/tokio-rs/axum)) and distributed as a Docker image.

---

## Features

- **REST API** — upload reports via `POST /api/reports/upload` with API key authentication
- **GitHub Action** — zero-config integration that zips results and posts a commit status
- **Bash script** — lightweight upload helper for any CI or local use
- **Allure 3 generation** — automatically runs `allure generate` after upload and tracks run history
- **Raw report hosting** — store and serve any pre-built HTML report as-is
- **Access control** — restrict the dashboard and report viewer to specific IPs or CIDR ranges
- **Swagger UI** — interactive API docs at `/swagger-ui`
- **500 MB upload limit**

---

## Quickstart

### 1. Run with Docker Compose

```sh
# Set your API key (the only required variable)
export API_SECRET=change-me

docker-compose up -d
```

The service listens on port `8080`. Reports are persisted in a named Docker volume.

To configure additional options, create `api/.env` from the example:

```sh
cp api/.env.example api/.env
# edit api/.env
```

---

### 2. Upload a Report via GitHub Actions

Add the following step to your workflow **after** your tests run:

```yaml
- uses: sireto/allure-report-host@v1
  with:
    serverUrl: ${{ vars.REPORT_SERVER_URL }}
    serverApiKey: ${{ secrets.REPORT_API_SECRET }}
    projectName: my-app
    branch: ${{ github.ref_name }}
    reportName: nightly            # logical name — run ID is tracked separately
    path: ./allure-results        # folder or .zip file
```

The action will:
1. Zip the results folder if a directory is provided.
2. Upload to the server.
3. Post a GitHub commit status with pass/fail counts and a direct link to the report.

#### All inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `serverUrl` | yes | — | Base URL of your report host (e.g. `https://reports.example.com`) |
| `serverApiKey` | yes | — | API key (`API_SECRET`) stored as a GitHub secret |
| `projectName` | yes | — | Logical project name (used as a path segment) |
| `branch` | yes | — | Branch name |
| `reportName` | yes | — | Logical report name (e.g. `nightly`, `smoke`). Do not include the run ID here — it is tracked separately via `run_id`. |
| `path` | yes | — | Path to an `allure-results` folder **or** a `.zip` file |
| `reportType` | no | `allure` | `allure` or `raw` |
| `githubToken` | no | `github.token` | Token used to post a commit status |
| `testResult` | no | — | Exit code of your test step; `0` = success, non-zero = failure |
| `testsPassed` | no | auto-detected | Number of passed tests (auto-counted from result JSONs if omitted) |
| `testsFailed` | no | auto-detected | Number of failed tests (auto-counted from result JSONs if omitted) |

---

### 3. Upload via Script

Use `scripts/publish-report.sh` from any shell or CI environment:

```sh
./scripts/publish-report.sh \
  --url 'https://reports.example.com' \
  --key 'your-api-key' \
  my-app main nightly-2024-06-20 allure ./allure-results
```

**Positional arguments (all optional — defaults shown):**

| Argument | Default | Description |
|---|---|---|
| `project` | basename of `$PWD` | Project name |
| `branch` | current git branch | Branch name |
| `report_name` | `<date>-<short-sha>` | Logical report name — do not include the run ID; use `--run-id` for that |
| `type` | `allure` | `allure` or `raw` |
| `path` | `./allure-results` | Folder or `.zip` to upload |

**Flags:**

| Flag | Description |
|---|---|
| `--url <url>` | Server base URL (required) |
| `--key <key>` | API key (required) |
| `--run-id <id>` | Custom run identifier |
| `--dry-run` | Print the `curl` command without executing it |
| `--verbose` / `-v` | Enable verbose output |

If `path` is a directory it is zipped automatically before upload.

---

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `API_SECRET` | **yes** | — | API key that clients must supply via `X-API-Key` header |
| `DATA_DIR` | no | `../data` | Directory where uploaded reports are stored |
| `RUST_ENV` | no | — | Set to `production` to disable `.env` file loading |
| `ALLOWED_IPS` | no | *(all)* | Comma-separated list of IPs/CIDRs allowed to view reports |
| `ALLOWED_PROXY_IPS` | no | *(none)* | Comma-separated list of trusted reverse-proxy IPs/CIDRs (for `X-Forwarded-For`) |

If `ALLOWED_IPS` is empty, the dashboard and report viewer are open to all clients. Upload endpoints always require a valid `X-API-Key`.

---

## Report Types

| Type | Behavior |
|---|---|
| `allure` | Extracts the uploaded ZIP, runs `allure generate`, and serves the HTML report. Run history is tracked in `history.jsonl`. |
| `raw` | Stores the uploaded ZIP and serves its contents as-is. Useful for pre-built HTML reports. |

---

## Storage Layout

```
data/
└── <project>/
    └── <branch>/
        └── <report_name>/
            └── <run_id>/
                ├── allure-results/   # extracted input
                └── allure-report/    # generated HTML (allure type only)
```

Reports are served as static files directly from the `DATA_DIR` volume, so any path under `data/` is accessible via the server URL.

---

## API Reference

Interactive API documentation is available at **`/swagger-ui`** when the service is running.

### `POST /api/reports/upload`

Upload a report as a `multipart/form-data` request.

| Field | Type | Required | Description |
|---|---|---|---|
| `project_name` | string | yes | Project name |
| `branch` | string | yes | Branch name |
| `report_name` | string | yes | Report name |
| `type` | string | no | `allure` (default) or `raw` |
| `run_id` | string | no | Unique run identifier (e.g. `<sha>/<run_id>`). Keeps individual runs of the same report name distinct. |
| `file` | binary | yes | `.zip` file containing `allure-results/` |

**Authentication:** `X-API-Key: <API_SECRET>` header is required on all requests.

**Size limit:** 500 MB per upload.

---

## Deployment

### Local (development)

```sh
export API_SECRET=change-me
docker-compose up -d
# → http://localhost:8080
```

## Building from Source

Prerequisites: Rust (stable toolchain), and `allure` CLI on `PATH` for report generation.

```sh
cd api
cargo build --release
./target/release/api
```

---

## Notes

- Uploaded ZIPs must contain an `allure-results/` directory at the top level for `allure` type reports.
- The `allure` CLI must be available in the container (or on the host) for report generation to succeed.
- History across runs is maintained via `history.jsonl` stored alongside each report.
- Only Allure 3 format is supported for report generation.
