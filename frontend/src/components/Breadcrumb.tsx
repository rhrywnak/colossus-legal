import React from "react";
import { Link } from "react-router-dom";

export type BreadcrumbItem = {
  label: string;
  to?: string;
};

const Breadcrumb: React.FC<{ items: BreadcrumbItem[] }> = ({ items }) => (
  <nav
    style={{
      fontSize: "0.85rem",
      color: "#6b7280",
      marginBottom: "1rem",
      display: "flex",
      flexWrap: "wrap",
      alignItems: "center",
      gap: "0.25rem",
    }}
  >
    {items.map((item, i) => (
      <React.Fragment key={item.label}>
        {i > 0 && <span style={{ margin: "0 0.25rem" }}>&gt;</span>}
        {item.to ? (
          <Link to={item.to} style={{ color: "#2563eb", textDecoration: "none" }}>
            {item.label}
          </Link>
        ) : (
          <span style={{ color: "#374151", fontWeight: 500 }}>{item.label}</span>
        )}
      </React.Fragment>
    ))}
  </nav>
);

export default Breadcrumb;
