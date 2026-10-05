// @vitest-environment jsdom
import { Background, Handle, Position, ReactFlow } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeAll, describe, expect, it, vi } from "vitest";

beforeAll(() => {
	class ResizeObserverStub {
		observe() {}
		unobserve() {}
		disconnect() {}
	}
	globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;

	globalThis.DOMMatrixReadOnly = class {
		m22 = 1;
	} as unknown as typeof DOMMatrixReadOnly;

	if (!window.matchMedia) {
		window.matchMedia = ((query: string) => ({
			matches: false,
			media: query,
			onchange: null,
			addListener: () => {},
			removeListener: () => {},
			addEventListener: () => {},
			removeEventListener: () => {},
			dispatchEvent: () => false,
		})) as unknown as typeof window.matchMedia;
	}
});

function InteractiveNode({ data }: { data: { onClick: () => void } }) {
	return (
		<div className="nodrag nopan nowheel">
			<Handle type="target" position={Position.Left} isConnectable={false} />
			<button type="button" onClick={data.onClick}>
				press me
			</button>
			<Handle type="source" position={Position.Right} isConnectable={false} />
		</div>
	);
}

const nodeTypes = { interactive: InteractiveNode };

function Harness({ onClick }: { onClick: () => void }) {
	const [nodes] = useState([
		{
			id: "a",
			type: "interactive",
			position: { x: 0, y: 0 },
			width: 200,
			height: 80,
			measured: { width: 200, height: 80 },
			draggable: false,
			selectable: false,
			data: { onClick },
		},
	]);

	return (
		<div style={{ width: 800, height: 600 }}>
			<ReactFlow
				nodes={nodes}
				edges={[]}
				nodeTypes={nodeTypes}
				nodesDraggable={false}
				nodesConnectable={false}
				elementsSelectable={false}
				panOnDrag={[1, 2]}
				zoomOnDoubleClick={false}
				onNodeMouseEnter={() => {}}
			>
				<Background />
			</ReactFlow>
		</div>
	);
}

describe("React Flow node interactivity", () => {
	it("fires a button click inside a custom node", async () => {
		const onClick = vi.fn();
		const user = userEvent.setup();

		render(<Harness onClick={onClick} />);

		const button = await screen.findByRole("button", { name: "press me" });
		await user.click(button);

		expect(onClick).toHaveBeenCalledTimes(1);
	});
});
