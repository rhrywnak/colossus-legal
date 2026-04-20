/**
 * AdminPrompts — Admin tab for managing prompt template files (.md).
 *
 * Thin wrapper over [`AdminFileManager`]; all behavior lives there.
 */
import React from "react";
import AdminFileManager, { FileManagerColumn } from "./AdminFileManager";
import {
  createTemplate,
  deleteTemplate,
  getTemplate,
  listTemplates,
  TemplateInfo,
  updateTemplate,
} from "../../services/configApi";

const columns: FileManagerColumn<TemplateInfo>[] = [
  { header: "Filename", render: (r) => r.filename, style: { width: "28%" } },
  {
    header: "Size",
    render: (r) => `${r.size_bytes.toLocaleString()} B`,
    style: { width: "100px" },
  },
  {
    header: "Preview",
    render: (r) => (
      <span style={{ color: "#64748b" }}>
        {r.preview.slice(0, 100)}
        {r.preview.length > 100 ? "…" : ""}
      </span>
    ),
  },
];

const AdminPrompts: React.FC = () => (
  <AdminFileManager<TemplateInfo>
    resourceLabel="Template"
    extension=".md"
    fetchList={() => listTemplates().then((r) => r.templates)}
    fetchItem={getTemplate}
    createItem={createTemplate}
    updateItem={updateTemplate}
    deleteItem={deleteTemplate}
    columns={columns}
    getFilename={(r) => r.filename}
  />
);

export default AdminPrompts;
