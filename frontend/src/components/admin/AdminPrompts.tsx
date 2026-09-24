/**
 * AdminPrompts — Admin tab for managing prompt template files (.md).
 *
 * Thin wrapper over [`AdminFileManager`]; all behavior lives there.
 */
import React, { useState } from "react";
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
      <span style={{ color: "var(--text-muted)" }}>
        {r.preview.slice(0, 100)}
        {r.preview.length > 100 ? "…" : ""}
      </span>
    ),
  },
];

/**
 * Read-only (ruling Q3): the instructions folder cannot be written by the app,
 * and new versions arrive with a release. The "Used for" column names the AI
 * jobs using each file; its heading and the tab's note come from the server
 * with the list, so neither is ever drawn blank.
 */
const AdminPrompts: React.FC = () => {
  const [meta, setMeta] = useState<{ note: string; usedFor: string } | null>(null);
  const withUsedFor: FileManagerColumn<TemplateInfo>[] = [
    ...columns,
    { header: meta?.usedFor ?? "", render: (r) => r.used_for ?? "\u2014", style: { width: "22%" } },
  ];
  return (
    <AdminFileManager<TemplateInfo>
      resourceLabel="Template"
      extension=".md"
      fetchList={() =>
        listTemplates().then((r) => {
          setMeta({ note: r.read_only_note, usedFor: r.used_for_label });
          return r.templates;
        })
      }
      fetchItem={getTemplate}
      createItem={createTemplate}
      updateItem={updateTemplate}
      deleteItem={deleteTemplate}
      columns={withUsedFor}
      getFilename={(r) => r.filename}
      readOnlyNote={meta?.note ?? ""}
    />
  );
};

export default AdminPrompts;
