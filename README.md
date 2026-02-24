# Allure Report Host

A self-hosted service for uploading, storing, and viewing Allure 3 test reports from any CI pipeline.

---

## Features

- Upload Allure reports via REST API, script, or GitHub Action
- Supports Allure 3 report generation
- Centralized dashboard for browsing reports

---

## 🚀 Quickstart

### 1. Run with Docker Compose

```sh
# Copy .env.example to .env and set your values
cp api/.env.example api/.env

# Start the service
docker-compose up -d
```

- The service will be available at [http://localhost:8080](http://localhost:8080) (or your configured port).
- Reports and data are stored in the `data/` directory by default.

---

### 2. Submit a Report from GitHub Actions

Add this step to your workflow after your tests:

```yaml
- uses: sireto/allure-report-host@v1
  with:
    serverUrl: ${{ vars.SERVER_URL }}
    serverApiKey: ${{ secrets.REPORT_API_SECRET }}
    projectName: my-app
    branch: ${{ github.ref_name }}
    reportName: nightly-${{ github.run_id }}
    # reportType: allure  # Optional, defaults to 'allure'
    path: ./allure-results
```

- `path` can be either the `allure-results` folder or a zipped file; the action will zip the folder if needed.
- The API key should be stored as a GitHub secret.
- `reportType` supports:
  - `allure` (default): For Allure 3 compatible results (recommended)
  - `raw`: For uploading raw test artifacts (they will be stored but not processed as Allure reports)
  - **Other values are not supported.** For generic HTML reports, use `raw` and upload a zipped folder containing your HTML files. These will be stored and served as-is, but will not appear as Allure reports in the dashboard.

---

## Requirements

- **Test report format:**  
  - If uploading **directly via the API**, you must upload a `.zip` file containing an `allure-results` folder with Allure-compatible JSON/XML files.
  - If using the provided **script** or **GitHub Action**, you can provide either the `allure-results` folder or a zipped file; the script/action will automatically zip the folder for you if needed.
  - Most test frameworks (Playwright, pytest, etc.) can output Allure results.
  - The service generates Allure 3 HTML reports from these results.

  > Need help generating Allure results?  
  > See the [Allure documentation](https://docs.qameta.io/allure/) for setup guides for Java, JavaScript, Python, and more.

- **Environment variables:**  
  See [`api/.env.example`](api/.env.example) for required variables:
  ```
  API_SECRET=your-api-key
  DATA_DIR=../data
  RUST_ENV=production
  ```

---

## Viewing Reports

After upload, view your reports at:

```
- Local: http://localhost:8080/
- Production: https://reports.sireto.io/
```
or your configured domain.

Browse by project, branch, and report name.

---

## API Reference

- [Swagger UI](http://localhost:8080/swagger-ui) for API documentation.

---

## API Authentication

- All uploads require an `X-API-Key` header.
- Use your API key from the environment or CI secrets.

---

## Environment Setup

- Copy [`api/.env.example`](api/.env.example) to `api/.env` and set your values.
- `API_SECRET` is required for authentication.
- `DATA_DIR` is the directory where reports are stored.

---

## Notes

- Uploaded ZIP files must contain Allure-compatible results (`allure-results` folder) for Allure reports.
- For generic HTML reports, use `reportType: raw` and upload a zipped folder containing your HTML files.
- The service generates Allure 3 HTML reports automatically after upload if the input is Allure-compatible.
- Only Allure 3 format is supported for report generation.
- Maximum upload size is 500MB.

---

## Uploading Reports via Script

You can also upload reports from CI or locally using the provided script:

```sh
./scripts/publish-report.sh \
  --url 'https://reports.sireto.io/api/reports/upload' \
  --key '<YOUR_API_KEY>' \
  <project> <branch> <report_name> <type> <path>
```

- `<project>`: Project name (e.g. `my-app`)
- `<branch>`: Branch name (e.g. `main`)
- `<report_name>`: Report name (e.g. `nightly-2024-06-20`)
- `<type>`: `allure` or `raw` (**default:** `allure`)
- `<path>`: Path to `allure-results` folder or zipped report

You can provide either the `allure-results` folder or a zipped file as the last argument.  
If you provide a folder, the script will automatically zip it before uploading.

**Example:**
```sh
./scripts/publish-report.sh \
  --url 'https://reports.sireto.io/api/reports/upload' \
  --key 'your-api-key' \
  my-app main nightly-2024-06-20 allure ./allure-results
```

---

**For any issues, check logs or the API documentation.**