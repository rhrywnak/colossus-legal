/**
 * AdminSchemas — Admin tab for managing extraction schema files (.yaml).
 *
 * Thin wrapper over [`AdminFileManager`]; the list view exposes the
 * schema metadata (document_type, version, entity_type_count) returned
 * by GET /schemas, while the edit view hands the raw YAML to a plain
 * textarea.
 */
import React from "react";
import AdminFileManager, { FileManagerColumn } from "./AdminFileManager";
import {
  createSchema,
  deleteSchema,
  getSchema,
  listSchemas,
  SchemaInfo,
  updateSchema,
} from "../../services/configApi";

const columns: FileManagerColumn<SchemaInfo>[] = [
  { header: "Filename", render: (r) => r.filename, style: { width: "25%" } },
  {
    header: "Document Type",
    render: (r) => r.document_type || "-",
    style: { width: "18%" },
  },
  {
    header: "Version",
    render: (r) => r.version || "-",
    style: { width: "80px" },
  },
  {
    header: "Entity Types",
    render: (r) => (
      <span>
        <span style={{ color: "#0f172a", fontWeight: 500 }}>
          {r.entity_type_count}
        </span>
        {r.entity_types.length > 0 && (
          <span style={{ color: "#64748b", marginLeft: "0.5rem" }}>
            {r.entity_types.slice(0, 3).join(", ")}
            {r.entity_types.length > 3 ? "…" : ""}
          </span>
        )}
      </span>
    ),
  },
];

const AdminSchemas: React.FC = () => (
  <AdminFileManager<SchemaInfo>
    resourceLabel="Schema"
    extension=".yaml"
    fetchList={() => listSchemas().then((r) => r.schemas)}
    fetchItem={getSchema}
    createItem={createSchema}
    updateItem={updateSchema}
    deleteItem={deleteSchema}
    columns={columns}
    getFilename={(r) => r.filename}
  />
);

export default AdminSchemas;
