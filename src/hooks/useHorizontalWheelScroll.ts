import { type RefObject, useEffect } from "react";

const VERTICAL_SCROLL_AREA_ATTRIBUTE = "data-vertical-scroll-area";

function canScrollVertically(element: Element): boolean {
	return element.scrollHeight > element.clientHeight;
}

function isInsideVerticallyScrollableArea(target: EventTarget | null, container: HTMLElement) {
	if (!(target instanceof Element)) return false;
	const area = target.closest(`[${VERTICAL_SCROLL_AREA_ATTRIBUTE}]`);
	return area !== null && container.contains(area) && canScrollVertically(area);
}

export function useHorizontalWheelScroll(containerRef: RefObject<HTMLElement | null>) {
	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		function handleWheel(event: WheelEvent) {
			if (!container) return;
			const isMostlyVertical = Math.abs(event.deltaY) > Math.abs(event.deltaX);
			if (!isMostlyVertical || event.ctrlKey) return;
			if (isInsideVerticallyScrollableArea(event.target, container)) return;

			event.preventDefault();
			container.scrollLeft += event.deltaY;
		}

		container.addEventListener("wheel", handleWheel, { passive: false });
		return () => container.removeEventListener("wheel", handleWheel);
	}, [containerRef]);
}
