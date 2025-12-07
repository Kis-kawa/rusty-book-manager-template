"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import Link from "next/link";

// 型定義
type Trip = {
trip_id: string;
source: string;
destination: string;
departure_time: string;
arrival_time: string;
vehicle_name: string;
status: string;
};

const formatDate = (dateString: string) => {
return new Date(dateString).toLocaleString('ja-JP', {
    month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit',
});
};

export default function AdminPage() {
const router = useRouter();
const [trips, setTrips] = useState<Trip[]>([]);
const [user, setUser] = useState<any>(null);

const loadData = async () => {
    // ... (ここは以前と同じログインチェック＆データ取得処理) ...
    const savedUser = localStorage.getItem("currentUser");
    if (!savedUser) {
    router.push("/login");
    return;
    }
    const userData = JSON.parse(savedUser);
    if (userData.role !== "admin") {
    alert("権限がありません");
    router.push("/");
    return;
    }
    setUser(userData);

    try {
    const res = await fetch("http://localhost:8000/trips");
    if (res.ok) setTrips(await res.json());
    } catch (e) { console.error(e); }
};

useEffect(() => { loadData(); }, [router]);

// ★更新後にリストを再読み込みする関数を子コンポーネントに渡す
const handleReload = () => loadData();

return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
    <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold text-red-500">運行管理システム (Admin)</h1>
        <Button variant="secondary" asChild><Link href="/">利用者画面へ戻る</Link></Button>
    </div>

    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {trips.map((trip) => (
        // カードの中身を別のコンポーネントとして切り出し
        <TripAdminCard key={trip.trip_id} trip={trip} user={user} onUpdate={handleReload} />
        ))}
    </div>
    </div>
);
}

// ----------------------------------------------------------------
// 個別のカードコンポーネント (ここにトグル機能を持たせる)
// ----------------------------------------------------------------
function TripAdminCard({ trip, user, onUpdate }: { trip: Trip, user: any, onUpdate: () => void }) {
const [isOpen, setIsOpen] = useState(false); // トグルの開閉状態
const [status, setStatus] = useState(trip.status); // 選択中のステータス
const [description, setDescription] = useState(""); // 説明文

// トグルを開いた時に初期値をセット
useEffect(() => {
    if (isOpen) {
    setStatus(trip.status);
    setDescription(""); // 説明文は毎回リセット（あるいはDBから取得して表示も可）
    }
}, [isOpen, trip.status]);

const handleApply = async () => {
    if (!confirm("この内容で更新しますか？")) return;

    try {
    const res = await fetch("http://localhost:8000/admin/status", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
        user_id: user.user_id,
        trip_id: trip.trip_id,
        status: status,
        description: description,
        }),
    });

    if (res.ok) {
        alert("更新しました");
        setIsOpen(false); // 閉じる
        onUpdate(); // 親のリストを更新
    } else {
        alert("更新エラー");
    }
    } catch (e) {
    alert("通信エラー");
    }
};

return (
    <Card className="bg-gray-800 border-gray-700 text-white">
    <CardHeader>
        <div className="flex justify-between items-center">
        <span className="text-gray-400 text-sm">{trip.vehicle_name}</span>
        <span className={`px-2 py-1 rounded text-xs font-bold
            ${trip.status === 'scheduled' ? 'bg-green-900 text-green-300' : ''}
            ${trip.status === 'delayed' ? 'bg-yellow-900 text-yellow-300' : ''}
            ${trip.status === 'cancelled' ? 'bg-red-900 text-red-300' : ''}
        `}>
            {trip.status.toUpperCase()}
        </span>
        </div>
        <CardTitle className="text-lg mt-2">{trip.source} → {trip.destination}</CardTitle>
    </CardHeader>

    <CardContent>
        <div className="mb-4 text-sm">
        <p>発: {formatDate(trip.departure_time)}</p>
        <p>着: {formatDate(trip.arrival_time)}</p>
        </div>

        {/* トグルボタン */}
        <Button
        variant={isOpen ? "secondary" : "default"}
        className="w-full mb-4"
        onClick={() => setIsOpen(!isOpen)}
        >
        {isOpen ? "閉じる" : "運行状況を変更する"}
        </Button>

        {/* 編集フォーム (isOpenがtrueの時だけ表示) */}
        {isOpen && (
        <div className="bg-gray-900 p-4 rounded border border-gray-600 flex flex-col gap-4">

            <div className="space-y-2">
            <Label>状態を選択</Label>
            <RadioGroup value={status} onValueChange={setStatus} className="flex gap-4">
                <div className="flex items-center space-x-2">
                <RadioGroupItem value="scheduled" id={`s-${trip.trip_id}`} className="text-green-500 border-green-500" />
                <Label htmlFor={`s-${trip.trip_id}`} className="text-green-400 cursor-pointer">通常</Label>
                </div>
                <div className="flex items-center space-x-2">
                <RadioGroupItem value="delayed" id={`d-${trip.trip_id}`} className="text-yellow-500 border-yellow-500" />
                <Label htmlFor={`d-${trip.trip_id}`} className="text-yellow-400 cursor-pointer">遅延</Label>
                </div>
                <div className="flex items-center space-x-2">
                <RadioGroupItem value="cancelled" id={`c-${trip.trip_id}`} className="text-red-500 border-red-500" />
                <Label htmlFor={`c-${trip.trip_id}`} className="text-red-400 cursor-pointer">運休</Label>
                </div>
            </RadioGroup>
            </div>

            {/* 遅延・運休の時だけテキスト入力表示 */}
            {(status === "delayed" || status === "cancelled") && (
            <div className="space-y-2 animate-in fade-in slide-in-from-top-2">
                <Label>理由・詳細 (利用者への通知に使われます)</Label>
                <Input
                placeholder="例: 雪の影響で30分遅れ / 車両故障のため"
                className="bg-gray-800 border-gray-600"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                />
            </div>
            )}

            <Button className="w-full bg-blue-600 hover:bg-blue-700" onClick={handleApply}>
            適用する
            </Button>
        </div>
        )}
    </CardContent>
    </Card>
);
}
