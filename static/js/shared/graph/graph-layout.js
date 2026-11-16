/* graph-layout.js*/

import elk from "./elk.js";

const GRAPH_PADDING = 64;

export async function layoutGraph(graph, container) {
    const positions = graph?.positions ?? [];
    const connections = graph?.connections ?? [];

    const nodes = positions.map(position => {
        const card = [...container.children].find(
            element => element.dataset.positionId === position.client_id
        );

        const rect = card?.getBoundingClientRect();

        return {
            id: position.client_id,
            width: Math.ceil(rect?.width ?? 220),
            height: Math.ceil(rect?.height ?? 190)
        };
    });

    const validNodeIds = new Set(nodes.map(node => node.id));

    const edges = connections
        .filter(connection =>
            validNodeIds.has(connection.from) &&
            validNodeIds.has(connection.to)
        )
        .map(connection => ({
            id: connection.client_id,
            sources: [connection.from],
            targets: [connection.to]
        }));

    const result = await elk.layout({
        id: "organization",
        layoutOptions: {
            "elk.algorithm": "layered",
            "elk.direction": "DOWN",
            "elk.edgeRouting": "ORTHOGONAL",
            "elk.spacing.nodeNode": "48",
            "elk.layered.spacing.nodeNodeBetweenLayers": "80",
            "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
            "elk.layered.nodePlacement.bk.fixedAlignment": "BALANCED",
            "elk.padding": `[top=${GRAPH_PADDING},left=${GRAPH_PADDING},bottom=${GRAPH_PADDING},right=${GRAPH_PADDING}]`
        },
        children: nodes,
        edges
    });

    return {
        width: Math.ceil(result.width ?? 0),
        height: Math.ceil(result.height ?? 0),
        nodes: result.children ?? [],
        edges: result.edges ?? []
    };
}