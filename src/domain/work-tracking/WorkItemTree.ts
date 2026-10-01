import type { WorkItemId } from "./identifiers";
import type { WorkItem } from "./WorkItem";

export type WorkItemNode = {
	readonly item: WorkItem;
	readonly children: WorkItemNode[];
};

function isAncestorOrSelf(
	candidate: WorkItemNode,
	start: WorkItemNode,
	parentByNode: ReadonlyMap<WorkItemNode, WorkItemNode>,
): boolean {
	let current: WorkItemNode | undefined = start;
	while (current !== undefined) {
		if (current === candidate) return true;
		current = parentByNode.get(current);
	}
	return false;
}

export function buildWorkItemTree(items: readonly WorkItem[]): WorkItemNode[] {
	const nodes = items.map((item): WorkItemNode => ({ item, children: [] }));
	const nodeById = new Map<WorkItemId, WorkItemNode>();
	for (const node of nodes) {
		if (!nodeById.has(node.item.id)) nodeById.set(node.item.id, node);
	}

	// Attaching only when the parent is not already a descendant keeps the graph a forest,
	// so a parent cycle (A -> B -> A) breaks at the later item instead of dropping both.
	const parentByNode = new Map<WorkItemNode, WorkItemNode>();
	for (const node of nodes) {
		const parentId = node.item.parentId;
		if (parentId === undefined || parentId === node.item.id) continue;
		const parent = nodeById.get(parentId);
		if (parent === undefined || isAncestorOrSelf(node, parent, parentByNode)) continue;
		parentByNode.set(node, parent);
	}

	const roots: WorkItemNode[] = [];
	for (const node of nodes) {
		const parent = parentByNode.get(node);
		if (parent === undefined) roots.push(node);
		else parent.children.push(node);
	}
	return roots;
}
