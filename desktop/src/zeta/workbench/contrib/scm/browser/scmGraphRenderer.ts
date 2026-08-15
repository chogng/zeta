import type { GitCommitSummary } from "../../../services/git/common/gitService.js";

const SvgNamespace = "http://www.w3.org/2000/svg";
const SwimlaneHeight = 22;
const SwimlaneWidth = 11;
const SwimlaneCurveRadius = 5;

interface ScmGraphRow {
  readonly commit: GitCommitSummary;
  readonly inputSwimlanes: readonly string[];
  readonly outputSwimlanes: readonly string[];
}

export type ScmGraphNodeKind = "commit" | "head" | "merge";

/** Projects ordered Git history into the swimlanes entering and leaving each row. */
export function createScmGraphRows(commits: readonly GitCommitSummary[]): readonly ScmGraphRow[] {
  const rows: ScmGraphRow[] = [];
  for (const commit of commits) {
    const inputSwimlanes = rows.at(-1)?.outputSwimlanes.slice() ?? [];
    const outputSwimlanes: string[] = [];
    let firstParentAdded = false;

    if (commit.parentObjectIds.length > 0) {
      for (const objectId of inputSwimlanes) {
        if (objectId === commit.objectId) {
          if (!firstParentAdded) {
            outputSwimlanes.push(commit.parentObjectIds[0]);
            firstParentAdded = true;
          }
          continue;
        }
        outputSwimlanes.push(objectId);
      }
    }

    for (let index = firstParentAdded ? 1 : 0; index < commit.parentObjectIds.length; index += 1) {
      outputSwimlanes.push(commit.parentObjectIds[index]);
    }
    rows.push({ commit, inputSwimlanes, outputSwimlanes });
  }
  return rows;
}

/** Renders one VS Code-compatible SCM history swimlane row. */
export function renderScmGraphRow(document: Document, row: ScmGraphRow, kind: ScmGraphNodeKind): SVGSVGElement {
  const svg = document.createElementNS(SvgNamespace, "svg");
  svg.classList.add("zeta-scm-graph-graph", kind);
  svg.setAttribute("aria-hidden", "true");
  const inputIndex = row.inputSwimlanes.findIndex((objectId) => objectId === row.commit.objectId);
  const circleIndex = inputIndex === -1 ? row.inputSwimlanes.length : inputIndex;
  let outputSwimlaneIndex = 0;

  for (let index = 0; index < row.inputSwimlanes.length; index += 1) {
    if (row.inputSwimlanes[index] === row.commit.objectId) {
      if (index !== circleIndex) {
        appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 A ${SwimlaneWidth} ${SwimlaneWidth} 0 0 1 ${SwimlaneWidth * index} ${SwimlaneWidth} H ${SwimlaneWidth * (circleIndex + 1)}`);
      } else {
        outputSwimlaneIndex += 1;
      }
      continue;
    }

    if (outputSwimlaneIndex >= row.outputSwimlanes.length || row.inputSwimlanes[index] !== row.outputSwimlanes[outputSwimlaneIndex]) continue;
    if (index === outputSwimlaneIndex) {
      appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 V ${SwimlaneHeight}`);
    } else {
      appendPath(svg, `M ${SwimlaneWidth * (index + 1)} 0 V 6 A ${SwimlaneCurveRadius} ${SwimlaneCurveRadius} 0 0 1 ${(SwimlaneWidth * (index + 1)) - SwimlaneCurveRadius} ${SwimlaneHeight / 2} H ${(SwimlaneWidth * (outputSwimlaneIndex + 1)) + SwimlaneCurveRadius} A ${SwimlaneCurveRadius} ${SwimlaneCurveRadius} 0 0 0 ${SwimlaneWidth * (outputSwimlaneIndex + 1)} ${(SwimlaneHeight / 2) + SwimlaneCurveRadius} V ${SwimlaneHeight}`);
    }
    outputSwimlaneIndex += 1;
  }

  for (let index = 1; index < row.commit.parentObjectIds.length; index += 1) {
    const parentOutputIndex = row.outputSwimlanes.lastIndexOf(row.commit.parentObjectIds[index]);
    if (parentOutputIndex === -1) continue;
    appendPath(svg, `M ${SwimlaneWidth * parentOutputIndex} ${SwimlaneHeight / 2} A ${SwimlaneWidth} ${SwimlaneWidth} 0 0 1 ${SwimlaneWidth * (parentOutputIndex + 1)} ${SwimlaneHeight} M ${SwimlaneWidth * parentOutputIndex} ${SwimlaneHeight / 2} H ${SwimlaneWidth * (circleIndex + 1)}`);
  }

  if (inputIndex !== -1) appendPath(svg, `M ${SwimlaneWidth * (circleIndex + 1)} 0 V ${SwimlaneHeight / 2}`);
  if (row.commit.parentObjectIds.length > 0) appendPath(svg, `M ${SwimlaneWidth * (circleIndex + 1)} ${SwimlaneHeight / 2} V ${SwimlaneHeight}`);
  appendNode(svg, circleIndex, kind);
  svg.style.width = `${SwimlaneWidth * (Math.max(row.inputSwimlanes.length, row.outputSwimlanes.length, 1) + 1)}px`;
  svg.style.height = `${SwimlaneHeight}px`;
  return svg;
}

function appendPath(svg: SVGSVGElement, data: string): void {
  const path = svg.ownerDocument.createElementNS(SvgNamespace, "path");
  path.classList.add("zeta-scm-graph-path");
  path.setAttribute("d", data);
  svg.append(path);
}

function appendNode(svg: SVGSVGElement, index: number, kind: ScmGraphNodeKind): void {
  if (kind === "head") {
    appendCircle(svg, index, 7, "outer");
    appendCircle(svg, index, 2, "inner");
    return;
  }
  if (kind === "merge") {
    appendCircle(svg, index, 6, "outer");
    appendCircle(svg, index, 3, "inner");
    return;
  }
  appendCircle(svg, index, 5, "single");
}

function appendCircle(svg: SVGSVGElement, index: number, radius: number, part: "inner" | "outer" | "single"): void {
  const circle = svg.ownerDocument.createElementNS(SvgNamespace, "circle");
  circle.classList.add("zeta-scm-graph-node", part);
  circle.setAttribute("cx", `${SwimlaneWidth * (index + 1)}`);
  circle.setAttribute("cy", `${SwimlaneHeight / 2}`);
  circle.setAttribute("r", `${radius}`);
  svg.append(circle);
}
