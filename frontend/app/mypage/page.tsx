"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import Link from "next/link";

// 型定義
type Reservation = {
reservation_id: string;
trip_id: string;
seat_number: number;
departure_time: string;
source: string;
destination: string;
vehicle_name: string;
};

const formatDate = (dateString: string) => {
return new Date(dateString).toLocaleString('ja-JP', {
    month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit', weekday: 'short',
});
};

export default function MyPage() {
const router = useRouter();
const [reservations, setReservations] = useState<Reservation[]>([]);
const [isLoading, setIsLoading] = useState(true);

useEffect(() => {
    // ログインチェック
    const savedUser = localStorage.getItem("currentUser");
    if (!savedUser) {
    alert("ログインしてください");
    router.push("/login");
    return;
    }
    const user = JSON.parse(savedUser);

    // 予約一覧を取得
    const fetchReservations = async () => {
    try {
        const res = await fetch("http://localhost:8000/my-reservations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ user_id: user.user_id }),
        });

        if (res.ok) {
        const data = await res.json();
        setReservations(data);
        }
    } catch (error) {
        console.error(error);
    } finally {
        setIsLoading(false);
    }
    };

    fetchReservations();
}, [router]);

return (
    <div className="min-h-screen bg-gray-50 p-8">
    <div className="max-w-4xl mx-auto">
        <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold text-gray-800">マイページ（予約履歴）</h1>
        <Button variant="outline" asChild>
            <Link href="/">ホームに戻る</Link>
        </Button>
        </div>

        {isLoading ? (
        <p>読み込み中...</p>
        ) : reservations.length === 0 ? (
        <div className="text-center py-20 text-gray-500">
            <p>まだ予約はありません。</p>
            <Button className="mt-4" asChild>
            <Link href="/">バスを予約する</Link>
            </Button>
        </div>
        ) : (
        <div className="grid gap-4">
            {reservations.map((res) => (
            <Card key={res.reservation_id} className="border-l-4 border-l-green-500">
                <CardHeader>
                <CardTitle className="flex justify-between items-center">
                    <span>{res.source} → {res.destination}</span>
                    <span className="text-lg bg-green-100 text-green-800 px-3 py-1 rounded-full">
                    {res.seat_number}号車
                    </span>
                </CardTitle>
                </CardHeader>
                <CardContent>
                <div className="flex gap-8 text-lg">
                    <div>
                    <span className="text-gray-500 text-sm block">出発日時</span>
                    <span className="font-bold">{formatDate(res.departure_time)}</span>
                    </div>
                    <div>
                    <span className="text-gray-500 text-sm block">車両</span>
                    <span>{res.vehicle_name}</span>
                    </div>
                </div>
                {/* ここにキャンセルボタンを追加予定 */}
                </CardContent>
            </Card>
            ))}
        </div>
        )}
    </div>
    </div>
);
}
