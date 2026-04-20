/**
 * AdminSystemPrompts — Admin tab for managing system-prompt files (.md).
 *
 * Thin wrapper over [`AdminFileManager`].
 */
import React from "react";
import AdminFileManager, { FileManagerColumn } from "./AdminFileManager";
import {
  createSystemPrompt,
  deleteSystemPrompt,
  getSystemPrompt,
  listSystemPrompts,
  SystemPromptInfo,
  updateSystemPrompt,
} from "../../services/configApi";

const columns: FileManagerColumn<SystemPromptInfo>[] = [
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

const AdminSystemPrompts: React.FC = () => (
  <AdminFileManager<SystemPromptInfo>
    resourceLabel="System Prompt"
    extension=".md"
    fetchList={() => listSystemPrompts().then((r) => r.system_prompts)}
    fetchItem={getSystemPrompt}
    createItem={createSystemPrompt}
    updateItem={updateSystemPrompt}
    deleteItem={deleteSystemPrompt}
    columns={columns}
    getFilename={(r) => r.filename}
  />
);

export default AdminSystemPrompts;
