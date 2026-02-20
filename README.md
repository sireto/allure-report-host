# Allure Report Host

A self-hosted service for uploading, storing, and viewing Allure 3 test reports from any CI pipeline.

---

## Features

- Upload Allure reports via REST API or script
- GitHub Action for CI integration
- Supports Allure 3 report generation
- Centralized dashboard for browsing reports

---

## Requirements

- **Test report format:** 
  - If uploading **directly via the API**, you must upload a `.zip` file containing an `allure-results` folder with Allure-compatible JSON/XML files.
  - If using the provided **script** or **GitHub Action**, you can provide either the `allure-results` folder or a zipped file; the script will automatically zip the folder for you if needed.
  - Most test frameworks (Playwright, pytest, etc.) can output Allure results.
  - The service generates Allure 3 HTML reports from these results.

- **Environment variables:**  
  See [`api/.env.example`](api/.env.example) for required variables:
  ```
  API_SECRET=your-api-key
  DATA_DIR=../data
  RUST_ENV=production
  ```

---

## Uploading Reports via Script


Use the provided script to upload reports from CI or locally:

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

**Example:**
```sh
./scripts/publish-report.sh \
  --url 'https://reports.sireto.io/api/reports/upload' \
  --key 'your-api-key' \
  my-app main nightly-2024-06-20 allure ./allure-results
```

---

## GitHub Action Usage


Integrate with your CI using the reusable action:

```yaml
- uses: sireto/allure-report-host@v1
  with:
    serverUrl: 'https://reports.sireto.io/api/reports/upload'
    serverApiKey: ${{ secrets.REPORT_API_KEY }}
    projectName: my-app
    branch: ${{ github.ref_name }}
    reportName: nightly-${{ github.run_id }}
    # reportType: allure  # Optional, defaults to 'allure' if omitted
    path: ./allure-results
```

**Note:** The `reportType` input defaults to `allure` if not specified.

---

## API Authentication

- All uploads require an `X-API-Key` header.
- Use your API key from the environment or CI secrets.

---

## Viewing Reports

After upload, view your reports at:

```
https://reports.sireto.io/
```

Browse by project, branch, and report name.

---

## API Reference

- [Swagger UI](https://reports.sireto.io/swagger-ui) for API documentation.

---

## Environment Setup

- Copy [`api/.env.example`](api/.env.example) to `api/.env` and set your values.
- `API_SECRET` is required for authentication.
- `DATA_DIR` is the directory where reports are stored.

---

## Notes

- Uploaded ZIP files must contain Allure-compatible results (`allure-results` folder).
- The service generates Allure 3 HTML reports automatically after upload.
- Only Allure 3 format is supported.
- Current Maximum upload is 500MB.

---

**For any issues, check logs or the API documentation.**