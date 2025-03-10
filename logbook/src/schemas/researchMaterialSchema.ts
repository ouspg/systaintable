const schema = {
    "type": "object",
    "properties": {
      "entry": {
        "type": "object",
        "properties": {
          "timestamp": { "type": "string", "format": "date-time" },
          "author": { "type": "string" },
          "processing type": {
            "type": "string",
            "enum": ["identification", "collection", "manual transformation", 
                    "automatic transformation", "audit", "analysis", "destruction", "other"]
          },
          "processing description": { "type": "string" }
        }
      },
      "data facts": {
        "type": "object",
        "properties": {
          "description": { "type": "string" },
          "storage": {
            "type": "object",
            "properties": {
              "source location": { "type": "string" },
              "location": { "type": "string" },
              "location other": { "type": "string" },
              "retention": {
                "type": "object",
                "properties": {
                  "deadline": { "type": "string", "format": "date" },
                  "removal policy": {
                    "type": "string",
                    "enum": ["delete", "delete and notify", "return and delete"]
                  }
                }
              }
            }
          },
          "metrics": {
            "type": "object",
            "properties": {
              "start time": { "type": "string", "format": "date-time" },
              "end time": { "type": "string", "format": "date-time" },
              "collection time": { "type": "string", "format": "date-time" },
              "size": { "type": "integer" },
              "event count": { "type": "integer" },
              "other metrics": { "type": "array", "items": { "type": "string" } }
            }
          },
          "rights": {
            "type": "object",
            "properties": {
              "license": { "type": "string" },
              "other license": { "type": "string" },
              "owner": {
                "type": "object",
                "properties": {
                  "owner name": { "type": "string" },
                  "contact name": { "type": "string" },
                  "contact email": { "type": "string", "format": "email" },
                  "contact phone": { "type": "string" },
                  "contact other": { "type": "string" },
                  "citation": { "type": "string" }
                }
              }
            }
          },
          "PII": {
            "type": "object",
            "properties": {
              "sanitation": {
                "type": "string",
                "enum": ["raw", "pseudonymized", "anonymized"]
              },
              "may contain": { "type": "array", "items": { "type": "string" } },
              "may contain other": { "type": "array", "items": { "type": "string" } },
              "confirmed to contain": { "type": "array", "items": { "type": "string" } },
              "confirmed to contain other": { "type": "array", "items": { "type": "string" } }
            }
          }
        }
      }
    }
  };
  
  export default schema;