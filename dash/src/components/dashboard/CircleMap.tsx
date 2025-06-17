import { useEffect, useMemo, useState } from "react";
import type { PositionCar } from "@/types/state.type";
import type { TrackPosition } from "@/types/map.type";
import { polarToCartesian } from "@/lib/circle";
import { fetchMap } from "@/lib/fetchMap";
import { useDataStore, usePositionStore } from "@/stores/useDataStore";
import { rotate, findMinDistance } from "@/lib/map";

export default function CircleMap() {
    const positions = usePositionStore((state) => state.positions);
    const drivers = useDataStore((state) => state?.driverList);
    const timingData = useDataStore((state) => state?.timingData);
    const circuitKey = useDataStore((state) => state?.sessionInfo?.meeting.circuit.key);

    const [points, setPoints] = useState<TrackPosition[] | null>(null);
    const [center, setCenter] = useState<[number, number]>([0, 0]);
    const [rotation, setRotation] = useState<number>(0);

    useEffect(() => {
        (async () => {
            if (!circuitKey) return;
            const mapJson = await fetchMap(circuitKey);
            if (!mapJson) return;
            const centerX = (Math.max(...mapJson.x) - Math.min(...mapJson.x)) / 2;
            const centerY = (Math.max(...mapJson.y) - Math.min(...mapJson.y)) / 2;
            const fixedRotation = mapJson.rotation + 90;
            const rotatedPoints = mapJson.x.map((x, index) => rotate(x, mapJson.y[index], fixedRotation, centerX, centerY));
            setPoints(rotatedPoints);
            setCenter([centerX, centerY]);
            setRotation(fixedRotation);
        })();
    }, [circuitKey]);

    const driverAngles = useMemo(() => {
        if (!points || !positions || !drivers)
            return [] as { angle: number; nr: string; color: string | undefined; tla: string; gap?: string }[];

        return Object.values(drivers)
            .filter((d) => positions[d.racingNumber])
            .map((d) => {
                const pos = positions[d.racingNumber] as PositionCar;
                const rotated = rotate(pos.X, pos.Y, rotation, center[0], center[1]);
                const idx = findMinDistance(rotated, points);
                const progress = idx / points.length;
                const angle = progress * 360;
                const gap = timingData?.lines[d.racingNumber]?.intervalToPositionAhead?.value;
                return { angle, nr: d.racingNumber, color: d.teamColour, tla: d.tla, gap };
            })
            .sort((a, b) => a.angle - b.angle);
    }, [points, positions, drivers, timingData, rotation, center]);

    if (!points) {
        return (
            <div className="h-full w-full p-2" style={{ minHeight: "35rem" }}>
                <div className="h-full w-full animate-pulse rounded-lg bg-zinc-800" />
            </div>
        );
    }

    const size = 200;
    const centerXY = size / 2;
    const radius = centerXY - 20;

    return (
        <svg viewBox={`0 0 ${size} ${size}`} className="h-full w-full xl:max-h-screen" xmlns="http://www.w3.org/2000/svg">
            <circle cx={centerXY} cy={centerXY} r={radius} className="stroke-zinc-700" fill="transparent" strokeWidth={2} />
            {driverAngles.map((d) => {
                const pos = polarToCartesian(centerXY, centerXY, radius, d.angle);
                return (
                    <g key={`circle.driver.${d.nr}`}> 
                        <circle cx={pos.x} cy={pos.y} r={4} fill={d.color ? `#${d.color}` : "#fff"} />
                        <text x={pos.x} y={pos.y - 6} textAnchor="middle" fontSize="10" className="fill-zinc-300">
                            {d.tla}
                        </text>
                    </g>
                );
            })}
            {driverAngles.map((d, idx) => {
                if (!d.gap) return null;
                const prev = idx === 0 ? driverAngles[driverAngles.length - 1] : driverAngles[idx - 1];
                let mid = (prev.angle + d.angle) / 2;
                if (idx === 0) {
                    const diff = d.angle + 360 - prev.angle;
                    mid = prev.angle + diff / 2;
                }
                const pos = polarToCartesian(centerXY, centerXY, radius + 12, mid);
                return (
                    <text key={`gap.${d.nr}`} x={pos.x} y={pos.y} textAnchor="middle" fontSize="8" className="fill-zinc-500">
                        {d.gap}
                    </text>
                );
            })}
        </svg>
    );
}
