// modelJobsStyles.ts — the jobs panel's and the Last document run tab's CSS.
//
// ## Why a CSS string and not inline style objects
//
// Board 4b (390 px) stacks each job — name, model, instructions, Save — and an
// inline `style={{}}` cannot hold a media query. The For-you page solved the
// same problem the same way (`forYouStyles.ts`, `PHONE_CSS`): one `<style>`
// block, every rule scoped under `[data-surface="admin-jobs"]` so nothing
// outside the two surfaces can be restyled by it.
//
// The rules are the mockup's own (MODEL_JOBS_OVERVIEW_MOCKUP_v3_2026-09-24,
// `.panel .phead .job3 .lbl .sel .menu .foot .fixed .btn .btn.off .meta .warn
// .links .cards .card table`), transcribed one for one, with its colours read
// from the scoped tokens in `styles/tokens.css`. Class names become `data-aj`
// attributes because this app styles by attribute, never by global class.

import type React from "react";

const S = '[data-surface="admin-jobs"]';

export const JOBS_CSS = `
${S}[data-aj="panel"]{border:1px solid var(--aj-line);border-radius:8px;margin-bottom:18px;background:var(--aj-paper);color:var(--aj-ink);font-size:15px;line-height:1.5}
${S} [data-aj="phead"]{padding:12px 16px;border-bottom:1px solid var(--aj-line)}
${S} [data-aj="phead-title"]{font-size:16px;font-weight:700;margin:0}
${S} [data-aj="phead-intro"]{color:var(--aj-muted);font-size:13px}
${S} [data-aj="job"]{display:grid;grid-template-columns:minmax(0,1.25fr) minmax(0,1fr) minmax(0,1.1fr) auto;gap:12px;align-items:center;padding:12px 16px;border-bottom:1px solid var(--aj-rule)}
${S} [data-aj="job"][data-last="true"]{border-bottom:0}
${S} [data-aj="jn"]{font-weight:600}
${S} [data-aj="jd"]{color:var(--aj-muted);font-size:13px}
${S} [data-aj="lbl"]{font-size:11.5px;color:var(--aj-muted);margin-bottom:2px}
${S} [data-aj="sel"]{width:100%;border:1px solid var(--aj-line);border-radius:6px;padding:7px 10px;display:flex;justify-content:space-between;gap:8px;background:var(--aj-paper);font:inherit;font-size:14px;color:var(--aj-ink);text-align:left;cursor:pointer}
${S} [data-aj="sel"] span:first-child{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
${S} [data-aj="sel"][aria-expanded="true"]{border-color:var(--aj-blue);box-shadow:0 0 0 2px var(--aj-ring)}
${S} [data-aj="menu"]{border:1px solid var(--aj-line);border-radius:6px;margin:4px 0 0;padding:0;list-style:none;background:var(--aj-paper);box-shadow:0 4px 14px rgba(0,0,0,.08);font-size:14px}
${S} [data-aj="menu"] li{padding:7px 10px;cursor:pointer}
${S} [data-aj="menu"] li[aria-selected="true"]{background:var(--aj-soft)}
${S} [data-aj="menu"] li[data-aj="foot"]{color:var(--aj-muted);font-size:12.5px;border-top:1px solid var(--aj-rule);cursor:default}
${S} [data-aj="fixed"]{background:var(--aj-fixed-bg);border:1px dashed var(--aj-line);border-radius:6px;padding:7px 10px;color:var(--aj-muted);font-size:14px}
${S} [data-aj="btn"]{background:var(--aj-blue);color:var(--aj-paper);border:0;border-radius:6px;padding:6px 14px;font:inherit;font-size:13px;font-weight:600;white-space:nowrap;cursor:pointer}
${S} [data-aj="btn"]:disabled{background:var(--aj-btn-off);cursor:default}
${S} [data-aj="meta"]{grid-column:1/-1;color:var(--aj-muted);font-size:12.5px;margin-top:-6px}
${S} [data-aj="warn"]{grid-column:1/-1;background:var(--aj-warn-bg);color:var(--aj-warn-ink);border:1px solid var(--aj-warn-border);border-radius:6px;padding:9px 12px;font-size:13px}
${S} [data-aj="links"]{padding:10px 16px;font-size:13px;color:var(--aj-muted);border-top:1px solid var(--aj-rule)}
${S} [data-aj="link"]{color:var(--aj-blue);font-weight:600;text-decoration:none}
${S} [data-aj="linkbtn"]{background:none;border:0;padding:0;font:inherit;color:var(--aj-blue);font-weight:600;cursor:pointer}
${S} [data-aj="linkbtn"]:disabled{color:var(--aj-muted);cursor:default}
${S} [data-aj="ok"]{background:var(--aj-okbg);color:var(--aj-ok);border-radius:6px;padding:10px 14px;margin:12px 16px 0;font-size:14px}
${S} [data-aj="others"]{padding:10px 16px;border-top:1px solid var(--aj-rule);font-size:13px;color:var(--aj-muted)}
${S} [data-aj="others"] b{color:var(--aj-ink);display:block;margin-bottom:4px}
${S}[data-aj="lastrun"]{color:var(--aj-ink);font-size:15px;line-height:1.5}
${S} [data-aj="cards"]{display:flex;gap:12px;flex-wrap:wrap;margin-bottom:14px}
${S} [data-aj="card"]{flex:1;min-width:150px;border:1px solid var(--aj-line);border-radius:8px;padding:12px 14px;background:var(--aj-paper)}
${S} [data-aj="card"] b{font-size:22px;display:block}
${S} [data-aj="card"] span{color:var(--aj-muted);font-size:13px}
${S} [data-aj="progress"]{font-size:14px;color:var(--aj-muted);margin:0 0 12px}
${S} [data-aj="table"]{width:100%;border-collapse:collapse;font-size:14px;background:var(--aj-paper)}
${S} [data-aj="table"] th{text-align:left;color:var(--aj-muted);font-weight:600;padding:9px 12px;background:var(--aj-pale);border-bottom:1px solid var(--aj-line)}
${S} [data-aj="table"] td{padding:9px 12px;border-bottom:1px solid var(--aj-rule)}
${S} [data-aj="summary"]{font-size:13px;color:var(--aj-muted)}
@media (max-width:820px){
  ${S} [data-aj="job"]{grid-template-columns:1fr}
  ${S} [data-aj="btn"]{justify-self:start}
}
`;

/** The one inline style left: the read-failure line, drawn outside the surface. */
export const errorLine: React.CSSProperties = {
  color: "var(--state-danger-strong)",
  fontSize: "0.84rem",
  margin: "0 0 1rem",
};
