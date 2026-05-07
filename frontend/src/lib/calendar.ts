export function formatDateTime(iso: string): string {
	const d = new Date(iso);
	return d.toLocaleDateString("en-US", {
		month: "short",
		day: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	});
}

export function getMonthDays(year: number, month: number): Date[] {
	const first = new Date(year, month, 1);
	const last = new Date(year, month + 1, 0);
	const days: Date[] = [];
	for (let i = first.getDay() - 1; i >= 0; i--)
		days.push(new Date(year, month, -i));
	for (let d = 1; d <= last.getDate(); d++) days.push(new Date(year, month, d));
	for (let i = 1; i <= 6 - last.getDay(); i++)
		days.push(new Date(year, month + 1, i));
	return days;
}

export function isToday(d: Date): boolean {
	const t = new Date();
	return (
		d.getFullYear() === t.getFullYear() &&
		d.getMonth() === t.getMonth() &&
		d.getDate() === t.getDate()
	);
}

export function isCurrentMonth(d: Date, y: number, m: number): boolean {
	return d.getFullYear() === y && d.getMonth() === m;
}

export const months = [
	"Jan",
	"Feb",
	"Mar",
	"Apr",
	"May",
	"Jun",
	"Jul",
	"Aug",
	"Sep",
	"Oct",
	"Nov",
	"Dec",
];
export const days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
