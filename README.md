# rapla-ical-proxy

This tool proxies requests to [DHBW](https://www.dhbw.de/english/home)'s HTML
[class schedule site](https://rapla.dhbw.de) into [ICS](https://icalendar.org)
calendars on the fly. This lets you import your class schedule into "real"
calendar software such such as Outlook, Google Calendar, etc. and keep
automatically receiving the latest schedule.

> [!TIP]
> If you study at DHBW and want to view your class schedule together with your
> work schedule in one and the same calendar app, **this is what you're looking
> for**.

## Fork

This fork contains a vibecoded web panel to fork rapla calendars and make modifications on your own. Das 'O' in DHBW steht für... if you know you know.

### Web Panel

| Path | Description |
|------|-------------|
| `/web` | Admin panel (Basic Auth required) — add calendars, modify/delete events, manage overlays |
| `/public/{id}` | Public calendar view — read-only FullCalendar rendering, no auth |
| `/api/calendars/{id}/forked.ics` | ICS feed for calendar subscription — public, no auth |

### Quick Start

1. Open `https://rapla.nulkode.dev/web` and log in with your credentials
2. Click **+ Add Calendar** and paste your Rapla URL
3. Click the calendar name to view it, or click **👁** to open the public view
4. Click **🔗** to copy the ICS subscription link
5. Subscribe to the ICS link in your calendar app

## Guide

Getting started is easy and requires zero setup if you use the official instance
at [rapla.nulkode.dev](https://rapla.nulkode.dev).

1. Get your Rapla link ready. This should be a decently long URL of the
   following shape:

  ```yaml
  https://rapla.dhbw.de/rapla/...
  ```

2. Replace the domain name `rapla.dhbw.de` with `rapla.nulkode.dev` (Or the
   hostname of another instance, see [self-hosting](#self-hosting)!). **Keep all
   other URL components the same!**

  ```diff
  - https://rapla.dhbw.de/rapla/rest
  + https://rapla.nulkode.dev/rapla/rest
  ```

3. Create a new calendar subscription in your calendar app. Paste in the
   modified URL. Done!

### Advanced Usage

By default, you will always receive any available events in within the `(now - 1
year, now + 1 year)` range.

If you'd like to avoid filling past calendar history with events beyond a
certain date, you can add the `cutoff_date` URL parameter:

```yaml
https://rapla.dhbw.de/rapla/calendar?other=parameters&cutoff_date=YYYY-MM-DD
```

This will shift the two-year range that is scanned by default to start at the
specified cutoff date.

## Self-hosting

The proxy is a simple single-binary webserver with no external dependencies.
You can deploy it on a VPS, serverless, or even on your local system directly in
front of your calendar software.

### Environment Variables

| Environment            | Default          | Description                                  |
| ---------------------- | ---------------- | -------------------------------------------- |
| `RAPLA_ADDRESS`        | `127.0.0.1:8080` | Socket address to listen at                  |
| `RAPLA_SERVER_URL`     | `http://<address>` | Public-facing URL (used in forked ICS links) |
| `RAPLA_CACHE_TTL`      | `3600` (1 hour)  | Time-to-live for cached calendars in seconds |
| `RAPLA_CACHE_MAX_SIZE` | `0`              | Maximum (estimated) cache size in Megabytes  |
| `RAPLA_DB_PATH`        | `rapla.db`       | Path to the SQLite database file             |
| `RAPLA_TAG`            | *(none)*         | Prefix for modified events, e.g. `CM` → `[CM] Math` |
| `RAPLA_WEB_USERNAME`   | *(required)*     | Basic Auth username for the admin panel      |
| `RAPLA_WEB_PASSWORD`   | *(required)*     | Basic Auth password for the admin panel      |

> [!NOTE]
> Setting `RAPLA_CACHE_MAX_SIZE` to `0` (the default) effectively disables
> caching. For production usage, I recommend allocating at least a couple of
> megabytes to caching. This saves a lot of network traffic and CPU time both on
> the proxy host and the upstream Rapla server.
