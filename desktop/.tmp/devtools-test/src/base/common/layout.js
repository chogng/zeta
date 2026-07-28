export var AnchorAlignment;
(function (AnchorAlignment) {
    AnchorAlignment[AnchorAlignment["Left"] = 0] = "Left";
    AnchorAlignment[AnchorAlignment["Right"] = 1] = "Right";
})(AnchorAlignment || (AnchorAlignment = {}));
export var AnchorPosition;
(function (AnchorPosition) {
    AnchorPosition[AnchorPosition["Below"] = 0] = "Below";
    AnchorPosition[AnchorPosition["Above"] = 1] = "Above";
})(AnchorPosition || (AnchorPosition = {}));
export var AnchorAxisAlignment;
(function (AnchorAxisAlignment) {
    AnchorAxisAlignment[AnchorAxisAlignment["Vertical"] = 0] = "Vertical";
    AnchorAxisAlignment[AnchorAxisAlignment["Horizontal"] = 1] = "Horizontal";
})(AnchorAxisAlignment || (AnchorAxisAlignment = {}));
/**
 * Places a rectangle next to an anchor, flipping the requested side or
 * alignment when that keeps more of the rectangle inside the viewport.
 *
 * The calculation is independent of DOM coordinate systems. Callers must
 * provide every rectangle in the same coordinate space.
 */
export function layout2d(viewport, view, anchor, options = {}) {
    const axis = options.anchorAxisAlignment ??
        AnchorAxisAlignment.Vertical;
    const gap = Math.max(0, options.gap ?? 0);
    const requestedPosition = options.anchorPosition ??
        AnchorPosition.Below;
    const requestedAlignment = options.anchorAlignment ??
        AnchorAlignment.Left;
    const primary = axis === AnchorAxisAlignment.Vertical
        ? placeBeside(viewport.top, viewport.height, view.height, anchor.top, anchor.height, requestedPosition, gap)
        : placeBeside(viewport.left, viewport.width, view.width, anchor.left, anchor.width, requestedPosition, gap);
    const cross = axis === AnchorAxisAlignment.Vertical
        ? alignWithAnchor(viewport.left, viewport.width, view.width, anchor.left, anchor.width, requestedAlignment)
        : alignWithAnchor(viewport.top, viewport.height, view.height, anchor.top, anchor.height, requestedAlignment);
    const left = axis === AnchorAxisAlignment.Vertical
        ? cross.offset
        : primary.offset;
    const top = axis === AnchorAxisAlignment.Vertical
        ? primary.offset
        : cross.offset;
    return {
        left,
        top,
        width: view.width,
        height: view.height,
        right: left + view.width,
        bottom: top + view.height,
        anchorAlignment: cross.alignment,
        anchorPosition: primary.position,
    };
}
function placeBeside(viewportStart, viewportSize, viewSize, anchorStart, anchorSize, requested, gap) {
    const viewportEnd = viewportStart + viewportSize;
    const anchorEnd = anchorStart + anchorSize;
    const after = anchorEnd + gap;
    const before = anchorStart - gap - viewSize;
    const fitsAfter = after + viewSize <= viewportEnd;
    const fitsBefore = before >= viewportStart;
    if (requested === AnchorPosition.Below) {
        if (fitsAfter) {
            return { offset: after, position: AnchorPosition.Below };
        }
        if (fitsBefore) {
            return { offset: before, position: AnchorPosition.Above };
        }
    }
    else {
        if (fitsBefore) {
            return { offset: before, position: AnchorPosition.Above };
        }
        if (fitsAfter) {
            return { offset: after, position: AnchorPosition.Below };
        }
    }
    const spaceAfter = Math.max(0, viewportEnd - after);
    const spaceBefore = Math.max(0, anchorStart - gap - viewportStart);
    const position = spaceAfter >= spaceBefore
        ? AnchorPosition.Below
        : AnchorPosition.Above;
    const desired = position === AnchorPosition.Below ? after : before;
    return {
        offset: clampToViewport(desired, viewSize, viewportStart, viewportEnd),
        position,
    };
}
function alignWithAnchor(viewportStart, viewportSize, viewSize, anchorStart, anchorSize, requested) {
    const viewportEnd = viewportStart + viewportSize;
    const leftAligned = anchorStart;
    const rightAligned = anchorStart + anchorSize - viewSize;
    const leftFits = leftAligned + viewSize <= viewportEnd &&
        leftAligned >= viewportStart;
    const rightFits = rightAligned >= viewportStart &&
        rightAligned + viewSize <= viewportEnd;
    if (requested === AnchorAlignment.Left) {
        if (leftFits) {
            return {
                offset: leftAligned,
                alignment: AnchorAlignment.Left,
            };
        }
        if (rightFits) {
            return {
                offset: rightAligned,
                alignment: AnchorAlignment.Right,
            };
        }
    }
    else {
        if (rightFits) {
            return {
                offset: rightAligned,
                alignment: AnchorAlignment.Right,
            };
        }
        if (leftFits) {
            return {
                offset: leftAligned,
                alignment: AnchorAlignment.Left,
            };
        }
    }
    const desired = requested === AnchorAlignment.Left
        ? leftAligned
        : rightAligned;
    return {
        offset: clampToViewport(desired, viewSize, viewportStart, viewportEnd),
        alignment: requested,
    };
}
function clampToViewport(offset, size, viewportStart, viewportEnd) {
    return Math.min(Math.max(offset, viewportStart), Math.max(viewportStart, viewportEnd - size));
}
