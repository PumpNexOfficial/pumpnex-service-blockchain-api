// OpenAPI schema generator
// Minimal implementation for GET /api/transactions endpoints

use serde_json::json;

pub fn generate_openapi_spec() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Blockchain API",
            "description": "Solana transaction indexing and querying API",
            "version": "0.2.0"
        },
        "servers": [
            {
                "url": "http://localhost:8080",
                "description": "Development server"
            }
        ],
        "paths": {
            "/api/transactions": {
                "get": {
                    "summary": "List transactions",
                    "description": "Get a paginated list of Solana transactions with optional filters",
                    "tags": ["transactions"],
                    "parameters": [
                        {
                            "name": "signature",
                            "in": "query",
                            "description": "Filter by transaction signature (exact match)",
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "from",
                            "in": "query",
                            "description": "Filter by source pubkey",
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "to",
                            "in": "query",
                            "description": "Filter by destination pubkey",
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "program_id",
                            "in": "query",
                            "description": "Filter by program ID (must be in program_ids array)",
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "slot_from",
                            "in": "query",
                            "description": "Filter by minimum slot number",
                            "schema": { "type": "integer", "format": "int64" }
                        },
                        {
                            "name": "slot_to",
                            "in": "query",
                            "description": "Filter by maximum slot number",
                            "schema": { "type": "integer", "format": "int64" }
                        },
                        {
                            "name": "sort_by",
                            "in": "query",
                            "description": "Sort field",
                            "schema": {
                                "type": "string",
                                "enum": ["slot", "signature", "block_time"],
                                "default": "slot"
                            }
                        },
                        {
                            "name": "order",
                            "in": "query",
                            "description": "Sort order",
                            "schema": {
                                "type": "string",
                                "enum": ["asc", "desc"],
                                "default": "desc"
                            }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "description": "Maximum number of results (1-200)",
                            "schema": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 200,
                                "default": 50
                            }
                        },
                        {
                            "name": "offset",
                            "in": "query",
                            "description": "Number of results to skip",
                            "schema": {
                                "type": "integer",
                                "minimum": 0,
                                "default": 0
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "headers": {
                                "ETag": {
                                    "description": "Entity tag for caching",
                                    "schema": { "type": "string" }
                                }
                            },
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/TransactionListResponse"
                                    }
                                }
                            }
                        },
                        "304": {
                            "description": "Not Modified (ETag matched)"
                        },
                        "400": {
                            "description": "Bad Request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        },
                        "503": {
                            "description": "Service Unavailable",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/transactions/{signature}": {
                "get": {
                    "summary": "Get transaction by signature",
                    "description": "Retrieve a single transaction by its signature",
                    "tags": ["transactions"],
                    "parameters": [
                        {
                            "name": "signature",
                            "in": "path",
                            "required": true,
                            "description": "Transaction signature",
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/SolanaTransaction"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Transaction not found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        },
                        "503": {
                            "description": "Service Unavailable",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/healthz": {
                "get": {
                    "summary": "Health check",
                    "description": "Simple liveness probe",
                    "tags": ["health"],
                    "responses": {
                        "200": {
                            "description": "Service is alive"
                        }
                    }
                }
            },
            "/readyz": {
                "get": {
                    "summary": "Readiness check",
                    "description": "Checks if service and integrations are ready",
                    "tags": ["health"],
                    "responses": {
                        "200": {
                            "description": "Service is ready"
                        },
                        "503": {
                            "description": "Service or integrations are not ready"
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "SolanaTransaction": {
                    "type": "object",
                    "properties": {
                        "signature": { "type": "string", "description": "Transaction signature (base58)" },
                        "slot": { "type": "integer", "format": "int64", "description": "Slot number" },
                        "from_pubkey": { "type": "string", "nullable": true, "description": "Source wallet" },
                        "to_pubkey": { "type": "string", "nullable": true, "description": "Destination wallet" },
                        "lamports": { "type": "integer", "format": "int64", "nullable": true, "description": "Amount in lamports" },
                        "program_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "nullable": true,
                            "description": "Program IDs involved"
                        },
                        "instructions": {
                            "type": "object",
                            "description": "Transaction instructions (JSONB)"
                        },
                        "block_time": { "type": "integer", "format": "int64", "nullable": true, "description": "Unix timestamp" },
                        "created_at": { "type": "string", "format": "date-time", "description": "Created timestamp" }
                    }
                },
                "TransactionListResponse": {
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SolanaTransaction"
                            }
                        },
                        "page": {
                            "type": "object",
                            "properties": {
                                "limit": { "type": "integer" },
                                "offset": { "type": "integer" },
                                "total": { "type": "integer", "format": "int64" }
                            }
                        },
                        "sort": {
                            "type": "object",
                            "properties": {
                                "by": { "type": "string" },
                                "order": { "type": "string" }
                            }
                        }
                    }
                },
                "ErrorResponse": {
                    "type": "object",
                    "properties": {
                        "error": { "type": "string" },
                        "details": { "type": "string", "nullable": true },
                        "missing": {
                            "type": "array",
                            "items": { "type": "string" },
                            "nullable": true
                        }
                    }
                }
            }
        }
    })
}

