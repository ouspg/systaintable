const uischema = {
    "type": "VerticalLayout",
    "elements": [
      {
        "type": "Group",
        "label": "Entry",
        "elements": [
          { "type": "Control", "scope": "#/properties/entry/properties/timestamp" },
          { "type": "Control", "scope": "#/properties/entry/properties/author" },
          { "type": "Control", "scope": "#/properties/entry/properties/processing type" },
          { "type": "Control", "scope": "#/properties/entry/properties/processing description" }
        ]
      },
      {
        "type": "Group",
        "label": "Data Facts",
        "elements": [
          { "type": "Control", "scope": "#/properties/data facts/properties/description" },
          {
            "type": "Group",
            "label": "Storage",
            "elements": [
              { "type": "Control", "scope": "#/properties/data facts/properties/storage/properties/source location" },
              { "type": "Control", "scope": "#/properties/data facts/properties/storage/properties/location" },
              { "type": "Control", "scope": "#/properties/data facts/properties/storage/properties/location other" },
              {
                "type": "Group",
                "label": "Retention",
                "elements": [
                  { "type": "Control", "scope": "#/properties/data facts/properties/storage/properties/retention/properties/deadline" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/storage/properties/retention/properties/removal policy" }
                ]
              }
            ]
          },
          {
            "type": "Group",
            "label": "Metrics",
            "elements": [
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/start time" },
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/end time" },
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/collection time" },
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/size" },
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/event count" },
              { "type": "Control", "scope": "#/properties/data facts/properties/metrics/properties/other metrics" }
            ]
          },
          {
            "type": "Group",
            "label": "Rights",
            "elements": [
              { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/license" },
              { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/other license" },
              {
                "type": "Group",
                "label": "Owner",
                "elements": [
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/owner name" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/contact name" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/contact email" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/contact phone" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/contact other" },
                  { "type": "Control", "scope": "#/properties/data facts/properties/rights/properties/owner/properties/citation" }
                ]
              }
            ]
          },
          {
            "type": "Group",
            "label": "PII",
            "elements": [
              { "type": "Control", "scope": "#/properties/data facts/properties/PII/properties/sanitation" },
              { "type": "Control", "scope": "#/properties/data facts/properties/PII/properties/may contain" },
              { "type": "Control", "scope": "#/properties/data facts/properties/PII/properties/may contain other" },
              { "type": "Control", "scope": "#/properties/data facts/properties/PII/properties/confirmed to contain" },
              { "type": "Control", "scope": "#/properties/data facts/properties/PII/properties/confirmed to contain other" }
            ]
          }
        ]
      }
    ]
  };
  
  export default uischema;