import type { Task, TaskId } from "@/bindings";

export type TaskNode = {
	readonly task: Task;
	readonly children: TaskNode[];
};

function isAncestorOrSelf(
	candidate: TaskNode,
	start: TaskNode,
	parentByNode: ReadonlyMap<TaskNode, TaskNode>,
): boolean {
	let current: TaskNode | undefined = start;
	while (current !== undefined) {
		if (current === candidate) return true;
		current = parentByNode.get(current);
	}
	return false;
}

export function buildTaskTree(tasks: readonly Task[]): TaskNode[] {
	const nodes = tasks.map((task): TaskNode => ({ task, children: [] }));
	const nodeById = new Map<TaskId, TaskNode>();
	for (const node of nodes) {
		if (!nodeById.has(node.task.id)) nodeById.set(node.task.id, node);
	}

	// Attaching only when the parent is not already a descendant keeps the graph a forest,
	// so a parent cycle (A -> B -> A) breaks at the later task instead of dropping both.
	const parentByNode = new Map<TaskNode, TaskNode>();
	for (const node of nodes) {
		const parentId = node.task.parentId;
		if (parentId === null || parentId === node.task.id) continue;
		const parent = nodeById.get(parentId);
		if (parent === undefined || isAncestorOrSelf(node, parent, parentByNode)) continue;
		parentByNode.set(node, parent);
	}

	const roots: TaskNode[] = [];
	for (const node of nodes) {
		const parent = parentByNode.get(node);
		if (parent === undefined) roots.push(node);
		else parent.children.push(node);
	}
	return roots;
}
