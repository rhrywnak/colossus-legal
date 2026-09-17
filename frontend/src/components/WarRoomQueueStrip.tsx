// =============================================================================
// WarRoomQueueStrip.tsx — the four queue tiles at the top of the War Room
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §5, ruled mockup v3 (GO v1 ruling 4: a new component
// at the top; `AlertsStrip` stays where it is). Props in, JSX out — every value
// and colour flag comes from `warRoomStripView`.

import React from "react";

import { warRoomStripStyle, warRoomTileStyle, warRoomTileValueStyle } from "./warRoomStripStyles";
import { warRoomStripView } from "./warRoomStripView";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";

const WarRoomQueueStrip: React.FC<{
  scenarios: ScenarioSummary[];
  wording: WarRoomWording;
}> = ({ scenarios, wording }) => {
  const tiles = warRoomStripView(scenarios, wording);
  return (
    <section style={warRoomStripStyle} data-strip="queues">
      {tiles.map((tile, i) => (
        <div key={tile.key} style={warRoomTileStyle(i === 0)} data-tile={tile.key}>
          <div style={warRoomTileValueStyle(tile)} data-warning={tile.warning}>
            {tile.value}
          </div>
          <div>{tile.label}</div>
        </div>
      ))}
    </section>
  );
};

export default WarRoomQueueStrip;
