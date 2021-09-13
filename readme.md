# borg-prometheus-exporter

A scraper for [Prometheus](https://prometheus.io/) for borg repositories.

## Usage

First, create a configuration file. An example configuration file is provided in `config.yml`. Then run the program with the path to that file as the first argument e.g. `./target/release/borg-prometheus-exporter config.yml`.

This may need to be run as root, as borg repositories' permissions are rather restrictive by default.

This program runs an http server, which Prometheus then polls. An example Prometheus configuration is provided:

```yml
scrape_configs:
  - job_name: "minecraft"
    scrape_timeout: 30s
    static_configs:
      - targets: ["localhost::9001"]
```

---

Available under the Mozilla Public Licence, version 2.0
