# Mermetro2

Mermetro2 is an additional tool for Mermetro (https://github.com/ouspg/systaintable/tree/main/mermetro). Both tools share the same logic in grouping, so the output will be same. This is a more detailed version of mermetro which focuses on **one ID** and shows only information of that ID.

See [mermetro/README.md](../mermetro/README.md) for more information.

## Project tree

```
mermetro/
├── mermetro2.py               # Main Python script
├── common_values.txt          # List of common values (e.g., DNS servers) to avoid unnecessary grouping
├── requirements.txt           # List of requirements
├── templates/
│   ├── group_timeline.html    # HTML template for the visualization frontend
├── data/                      # Safe path for data files (data/* in gitignore)
└── static/
    ├── favicon.ico            # Site icon

```

## Usage

Install requirements

```console
pip install -r requirements.txt
```

 Place the `.json` file exported from the Classifier into the `mermetro2/data/` directory

Run `mermetro2.py` as in the example below and wait, it can take several minutes to complete with larger files. Switch -m enables multiprocessing. 

```console
python3 mermetro.py2 data/lokitiedosto.json
```
OR
```console
python3 mermetro.py2 data/lokitiedosto.json -m
```

Open http://localhost:5000

### OSX venv

1. Create a virtual environment
```python3 -m venv venv```

2. Activate the virtual environment
```source venv/bin/activate```

3. Install pytz (and any other dependencies)
```pip install flask```

4. Run mermetro
```python mermetro2.py```

5. To deactivate the virtual environment when done:
```deactivate```
