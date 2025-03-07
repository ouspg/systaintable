import React, { useState } from "react";
import { JsonForms } from "@jsonforms/react";
import { materialCells, materialRenderers } from "@jsonforms/material-renderers";
import { Button } from "@mui/material";

const schema = {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "entry": {
      "type": "object",
      "properties": {
        "timestamp": { "type": "string", "format": "date-time" },
        "author": { "type": "string" },
        "processing_type": {
          "type": "string",
          "enum": [
            "identification", "collection", "manual transformation", "automatic transformation", "audit", "analysis", "destruction", "other"
          ]
        },
        "processing_description": { "type": "string" }
      }
    },
    "data_facts": {
      "type": "object",
      "properties": {
        "description": { "type": "string" },
        "storage": {
          "type": "object",
          "properties": {
            "source_location": { "type": "string" },
            "source_location_other": { "type": "string" },
            "location": { "type": "string" },
            "location_other": { "type": "string" },
            "retention": {
              "type": "object",
              "properties": {
                "deadline": { "type": "string", "format": "date" },
                "removal_policy": {
                  "type": "string", "enum": ["delete", "delete and notify", "return and delete"]
                }
              }
            }
          }
        },
        "metrics": {
          "type": "object",
          "properties": {
            "start_time": { "type": "string", "format": "date-time" },
            "end_time": { "type": "string", "format": "date-time" },
            "collection_time": { "type": "string" },
            "size": { "type": "string" },
            "event_count": { "type": "integer" },
            "other_metrics": { "type": "string" }
          }
        },
        "rights": {
          "type": "object",
          "properties": {
            "license": { "type": "string" },
            "other_license": { "type": "string" },
            "owner": {
              "type": "object",
              "properties": {
                "owner_name": { "type": "string" },
                "contact_name": { "type": "string" },
                "contact_email": { "type": "string", "format": "email" },
                "contact_phone": { "type": "string" },
                "contact_other": { "type": "string" },
                "citation": { "type": "string" }
              }
            }
          }
        },
        "PII": {
          "type": "object",
          "properties": {
            "sanitation": { "type": "string" },
            "may_contain": { "type": "string" },
            "may_contain_other": { "type": "string" },
            "confirmed_to_contain": { "type": "string" },
            "confirmed_to_contain_other": { "type": "string" }
          }
        }
      }
    }
  }
};

const uischema = {
  "type": "VerticalLayout",
  "elements": [
    {
      "type": "Group",
      "label": "Entry Details",
      "elements": [
        { "type": "Control", "scope": "#/properties/entry/properties/timestamp" },
        { "type": "Control", "scope": "#/properties/entry/properties/author" },
        { "type": "Control", "scope": "#/properties/entry/properties/processing_type" },
        { "type": "Control", "scope": "#/properties/entry/properties/processing_description" }
      ]
    },
    {
      "type": "Group",
      "label": "Data Facts",
      "elements": [
        { "type": "Control", "scope": "#/properties/data_facts/properties/description" },
        {
          "type": "Group",
          "label": "Storage",
          "elements": [
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/source_location" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/source_location_other" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/location" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/location_other" }
          ]
        },
        {
          "type": "Group",
          "label": "Retention",
          "elements": [
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/retention/properties/deadline" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/storage/properties/retention/properties/removal_policy" }
          ]
        },
        {
          "type": "Group",
          "label": "Metrics",
          "elements": [
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/start_time" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/end_time" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/collection_time" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/size" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/event_count" },
            { "type": "Control", "scope": "#/properties/data_facts/properties/metrics/properties/other_metrics" }
          ]
        }
      ]
    },
    {
      "type": "Group",
      "label": "Rights",
      "elements": [
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/license" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/other_license" }
      ]
    },
    {
      "type": "Group",
      "label": "Owner Details",
      "elements": [
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/owner_name" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/contact_name" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/contact_email" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/contact_phone" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/contact_other" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/rights/properties/owner/properties/citation" }
      ]
    },
    {
      "type": "Group",
      "label": "PII",
      "elements": [
        { "type": "Control", "scope": "#/properties/data_facts/properties/PII/properties/sanitation" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/PII/properties/may_contain" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/PII/properties/may_contain_other" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/PII/properties/confirmed_to_contain" },
        { "type": "Control", "scope": "#/properties/data_facts/properties/PII/properties/confirmed_to_contain_other" }
      ]
    }
  ]
};

const App = () => {
  const [data, setData] = useState({});

  const handleExport = () => {
    const jsonData = JSON.stringify(data, null, 2);
    const blob = new Blob([jsonData], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "research_data.json";
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      <h1>Research Material Data Form</h1>
      <JsonForms
        schema={schema}
        uischema={uischema}
        data={data}
        renderers={materialRenderers}
        cells={materialCells}
        onChange={({ data }) => setData(data)}
      />
      <Button variant="contained" color="primary" onClick={handleExport}>
        Export JSON
      </Button>
    </div>
  );
};

export default App;