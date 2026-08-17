import type { GitCommitSummary } from "../../../services/git/common/gitService.js";

const SvgNamespace = "http://www.w3.org/2000/svg";
const SwimlaneHeight = 22;
const SwimlaneWidth = 11;
const SwimlaneCurveRadius = 5;
const LaneColorCount = 8;

interface ScmGraphLane {
  readonly objectId: string;
  readonly colorIndex: number;
}

interface ScmGraphRow {
  readonly commit: GitCommitSummary;
  readonly inputSwimlanes: readonly ScmGraphLane[];
  readonly outputSwimlanes: readonly ScmGraphLane[];
}

export type ScmGraphNodeKind = "commit" | "head" | "merge";

/** Projects ordered Git history into stable, color-carrying swimlanes. */
export function createScmGraphRows(commits: readonly GitCommitSummary[]): readonly ScmGraphRow[] {
  const rows: ScmGraphRow[] = [];
  let nextColorIndex = 0;
  const allocateColor = (): number => {
    const colorIndex = nextColorIndex;
    nextColorIndex = (nextColorIndex + 1) % LaneColorCount;
    return colorIndex;
  };

  for (const commit of commits) {
    const inputSwimlanes = rows.at(-1)?.outputSwimlanes.map((lane) => ({ ...lane })) ?? [];
    const outputSwimlanes: ScmGraphLane[] = [];
    let firstParentAdded = false;
    const inputIndex = inputSwimlanes.findIndex((lane) => lane.objectId === commit.objectId);

    if (commit.parentObjectIds.length > 0) {
      for (const lane of inputSwimlanes) {
        if (lane.objectId === commit.objectId) {
          if (!firstParentAdded) {
            outputSwimlanes.push({ objectId: commit.parentObjectIds[0], colorIndex: lane.colorIndex });
            firstParentAdded = true;
          }
          continue;
        }
        outputSwimlanes.push({ ...lane });
      }
    }

    for (let index = firstParentAdded ? 1 : 0; index < commit.parentObjectIds.length; index += 1) {
      outputSwimlanes.push({ objectId: commit.parentObjectIds[index], colorIndex: allocateColor() });
    }
    rows.push({ commit, inputSwimlanes, outputSwimlanes });
  }
  return rows;
}

/** Renders one SCM history swimlane row with a stable color for each branch lane. */
export function renderScmGraphRow(document: Document, row: ScmGraphRow, kind: ScmGraphNodeKind): SVGSVGElement {
  const svg = document.createElementNS(SvgNamespace, "svg");
  svg.classList.add("zeta-scm-graph-graph", kind);
  svg.setAttribute("aria-hidden", "true");
  const inputIndex = row.inputSwimlanes.findIndex((lane) => lane.objectId === row.commit.objectId);
  const circleIndex = inputIndex === -1 ? row.inputSwimlanes.length : inputIndex;
  const circleColorIndex = inputIndex === -1 ? row.outputSwimlanes[0]?.colorIndex ?? 0 : row.inputSwimlanes[inputIndex].colorIndex;
  let outputSwimlaneIndex = 0;

  for (let index = 0; index < row.inputSwimlanes.length; index += 1) {
    const inputLane = row.inputSwimlanes[index];
    if (inputLane.objectId === row.commit.objectId) {
      if (index !== circleIndex) {
        appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 A ${SwimlaneWidth} ${SwimlaneWidth} 0 0 1 ${SwimlaneWidth * index} ${SwimlaneWidth} H ${SwimlaneWidth * (circleIndex + 1)}`, inputLane.colorIndex);
      } else {
        outputSwimlaneIndex += 1;
      }
      continue;
    }

    if (outputSwimlaneIndex >= row.outputSwimlanes.length || inputLane.objectId !== row.outputSwimlanes[outputSwimlaneIndex].objectId) continue;
    if (index === outputSwimlaneIndex) {
      appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 V ${SwimlaneHeight}`, inputLane.colorIndex);
    } else {
      appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 V 6 A ${SwimlaneCurveRadius} ${SwimlaneCurveRadius} 0 0 1 ${(SwimlaneWidth * (index + 1)) - SwimlaneCurveRadius} ${SwimlaneHeight / 2} H ${(SwimlaneWidth * (outputSwimlaneIndex + 1)) + SwimlaneCurveRadius} A ${SwimlaneCurveRadius} ${SwimlaneCurveRadius} 0 0 0 ${SwimlaneWidth * (outputSwimlaneIndex + 1)} ${(SwimlaneHeight / 2) + SwimlaneCurveRadius} V ${SwimlaneHeight}`, inputLane.colorIndex);
    }
    outputSwimlaneIndex += 1;
  }

  for (let index = 1; index < row.commit.parentObjectIds.length; index += 1) {
    const parentOutputIndex = row.outputSwimlanes.findIndex((lane) => lane.objectId === row.commit.parentObjectIds[index]);
    if (parentOutputIndex === -1) continue;
    const parentColorIndex = row.outputSwimlanes[parentOutputIndex].colorIndex;
    appendPath(svg, `M ${SwimlaneWidth * parentOutputIndex} ${SwimlaneHeight / 2} A ${SwimlaneWidth} ${SwimlaneWidth} 0 0 1 ${SwimlaneWidth * (parentOutputIndex + 1)} ${SwimlaneHeight}`, parentColorIndex);
    appendPath(svg, `M ${SwimlaneWidth * parentOutputIndex} ${SwimlaneHeight / 2} H ${SwimlaneWidth * (circleIndex + 1)}`, parentColorIndex);
  }

  if (inputIndex !== -1) appendPath(svg, `M ${SwimlaneWidth * (circleIndex + 1)} 0 V ${SwimlaneHeight / 2}`, circleColorIndex);
  if (row.commit.parentObjectIds.length > 0) appendPath(svg, `M ${SwimlaneWidth * (circleIndex + 1)} ${SwimlaneHeight / 2} V ${SwimlaneHeight}`, circleColorIndex);
  appendNode(svg, circleIndex, kind, circleColorIndex);
  svg.style.width = `${SwimlaneWidth * (Math.max(row.inputSwimlanes.length, row.outputSwimlanes.length, 1) + 1)}px`;
  svg.style.height = `${SwimlaneHeight}px`;
  return svg;
}

function appendPath(svg: SVGSVGElement, data: string, colorIndex: number): void {
  const path = svg.ownerDocument.createElementNS(SvgNamespace, "path");
  path.classList.add("zeta-scm-graph-path");
  path.dataset.laneColor = String(colorIndex);
  path.setAttribute("d", data);
  svg.append(path);
}

function appendNode(svg: SVGSVGElement, index: number, kind: ScmGraphNodeKind, colorIndex: number): void {
  if (kind === "head") {
    appendCircle(svg, index, 7, "outer", colorIndex);
    appendCircle(svg, index, 2, "inner", colorIndex);
    return;
  }
  if (kind === "merge") {
    appendCircle(svg, index, 6, "outer", colorIndex);
    appendCircle(svg, index, 3, "inner", colorIndex);
    return;
  }
  appendCircle(svg, index, 5, "single", colorIndex);
}

function appendCircle(svg: SVGSVGElement, index: number, radius: number, part: "inner" | "outer" | "single", colorIndex: number): void {
  const circle = svg.ownerDocument.createElementNS(SvgNamespace, "circle");
  circle.classList.add("zeta-scm-graph-node", part);
  circle.dataset.laneColor = String(colorIndex);
  circle.setAttribute("cx", `${SwimlaneWidth * (index + 1)}`);
  circle.setAttribute("cy", `${SwimlaneHeight / 2}`);
  circle.setAttribute("r", `${radius}`);
  svg.append(circle);
}
