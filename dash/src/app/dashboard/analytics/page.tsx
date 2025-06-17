"use client";

import { useEffect, useState } from "react";
import { Line } from "react-chartjs-2";
import {
	Chart as ChartJS,
	CategoryScale,
	LinearScale,
	PointElement,
	LineElement,
	Title,
	Tooltip,
	Legend,
} from "chart.js";

import { env } from "@/env";
import { useDataStore } from "@/stores/useDataStore";
import Select from "@/components/ui/Select";

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend);

interface Laptime {
	lap: number | null;
	time: string;
	laptime: number;
}

interface Gap {
	time: string;
	gap: number;
}

export default function AnalyticsPage() {
	const drivers = useDataStore((state) => state.driverList);
	const [driver, setDriver] = useState<string | null>(null);
	const [laptimes, setLaptimes] = useState<Laptime[]>([]);
	const [gaps, setGaps] = useState<Gap[]>([]);

	useEffect(() => {
		if (!driver) return;

		(async () => {
			try {
				const [ltRes, gapRes] = await Promise.all([
					fetch(`${env.NEXT_PUBLIC_ANALYTICS_URL}/api/laptime/${driver}`),
					fetch(`${env.NEXT_PUBLIC_ANALYTICS_URL}/api/gap/${driver}`),
				]);

				if (ltRes.ok) {
					const data: Laptime[] = await ltRes.json();
					setLaptimes(data);
				}
				if (gapRes.ok) {
					const data: Gap[] = await gapRes.json();
					setGaps(data);
				}
			} catch (e) {
				console.error("failed to fetch analytics", e);
			}
		})();
	}, [driver]);

	const driverOptions = drivers
		? Object.values(drivers).map((d) => ({ label: d.fullName, value: d.racingNumber }))
		: [];

	const laptimeData = {
		labels: laptimes.map((l) => l.lap ?? 0),
		datasets: [
			{
				label: "Lap Time (ms)",
				data: laptimes.map((l) => l.laptime),
				borderColor: "rgb(75, 192, 192)",
				backgroundColor: "rgba(75, 192, 192, 0.4)",
			},
		],
	};

	const gapData = {
		labels: gaps.map((g) => new Date(g.time).toLocaleTimeString()),
		datasets: [
			{
				label: "Gap to Leader (ms)",
				data: gaps.map((g) => g.gap),
				borderColor: "rgb(255, 99, 132)",
				backgroundColor: "rgba(255, 99, 132, 0.4)",
			},
		],
	};

	return (
		<div className="flex flex-col gap-4 p-4">
			<div className="w-64">
				<Select placeholder="Select driver" options={driverOptions} selected={driver} setSelected={setDriver} />
			</div>

			{driver && (
				<div className="grid grid-cols-1 gap-4 md:grid-cols-2">
					<Line data={laptimeData} />
					<Line data={gapData} />
				</div>
			)}
		</div>
	);
}
