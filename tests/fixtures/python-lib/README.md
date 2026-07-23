# python-lib

A tiny single-package Python library used as a decon-rs test fixture.

## Prerequisites

- Python 3.10 or newer
- `pip` or `venv` available

## Install

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Set `DATABASE_URL` and `API_KEY` in `.env` before running.

## Run

```bash
python -m mylib.cli --help
```
