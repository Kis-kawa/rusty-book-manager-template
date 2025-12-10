"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import Link from "next/link";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";


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

// 更新後にリストを再読み込みする関数を子コンポーネントに渡す
const handleReload = () => loadData();

return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
    <div className="flex justify-between items-center mb-8">
        <div className="flex items-center gap-4">
        <h1 className="text-3xl font-bold text-red-500">運行管理システム (Admin)</h1>
        <AddTripDialog user={user} onCreated={loadData} />
        </div>
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


// ----------------------------------------------------------------
// 新規便追加ダイアログ
// ----------------------------------------------------------------
function AddTripDialog({ user, onCreated }: { user: any, onCreated: () => void }) {
const [isOpen, setIsOpen] = useState(false);
const [options, setOptions] = useState<any>(null); // ルートや車両の選択肢

// フォームの状態
const [routeId, setRouteId] = useState("");
const [vehicleId, setVehicleId] = useState("");
const [driverId, setDriverId] = useState("");
const [departureTime, setDepartureTime] = useState("");
const [arrivalTime, setArrivalTime] = useState("");

// ダイアログが開いた時に選択肢データを取得
useEffect(() => {
    if (isOpen && user) {
    fetch("http://localhost:8000/admin/options", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ user_id: user.user_id }),
    })
        .then((res) => res.json())
        .then((data) => setOptions(data))
        .catch((e) => console.error(e));
    }
}, [isOpen, user]);

const handleSubmit = async () => {
    if (!routeId || !vehicleId || !driverId || !departureTime || !arrivalTime) {
    alert("すべての項目を入力してください");
    return;
    }

    try {
    const res = await fetch("http://localhost:8000/admin/trips", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
        user_id: user.user_id,
        route_id: routeId,
        vehicle_id: vehicleId,
        driver_id: driverId,
        departure_datetime: departureTime + ":00",
        arrival_datetime: arrivalTime + ":00",
        }),
    });

    if (res.ok) {
        alert("便を作成しました！");
        setIsOpen(false);
        // フォームリセット
        setRouteId("");
        setVehicleId("");
        setDriverId("");
        setDepartureTime("");
        setArrivalTime("");
        onCreated(); // リスト更新
    } else {
        alert("作成エラー");
    }
    } catch (e) {
    console.error(e);
    alert("通信エラー");
    }
};

return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
    <DialogTrigger asChild>
        <Button className="bg-blue-600 hover:bg-blue-500">＋ 新規便追加</Button>
    </DialogTrigger>
    <DialogContent className="bg-gray-800 text-white border-gray-700">
        <DialogHeader>
        <DialogTitle>新しい運行便を登録</DialogTitle>
        <DialogDescription>
            ルート、車両、日時を指定してスケジュールを追加します。
        </DialogDescription>
        </DialogHeader>

        {!options ? (
        <p>データを読み込み中...</p>
        ) : (
        <div className="grid gap-4 py-4">
            {/* ルート選択 */}
            <div className="grid gap-2">
            <Label>ルート</Label>
            <Select onValueChange={setRouteId} value={routeId}>
                <SelectTrigger className="bg-gray-700 border-gray-600">
                <SelectValue placeholder="ルートを選択" />
                </SelectTrigger>
                <SelectContent>
                {options.routes.map((r: any) => (
                    <SelectItem key={r.route_id} value={r.route_id}>{r.name}</SelectItem>
                ))}
                </SelectContent>
            </Select>
            </div>

            {/* 車両選択 */}
            <div className="grid gap-2">
            <Label>車両</Label>
            <Select onValueChange={setVehicleId} value={vehicleId}>
                <SelectTrigger className="bg-gray-700 border-gray-600">
                <SelectValue placeholder="車両を選択" />
                </SelectTrigger>
                <SelectContent>
                {options.vehicles.map((v: any) => (
                    <SelectItem key={v.id} value={v.id}>{v.name}</SelectItem>
                ))}
                </SelectContent>
            </Select>
            </div>

            {/* 運転手選択 */}
            <div className="grid gap-2">
            <Label>運転手</Label>
            <Select onValueChange={setDriverId} value={driverId}>
                <SelectTrigger className="bg-gray-700 border-gray-600">
                <SelectValue placeholder="運転手を選択" />
                </SelectTrigger>
                <SelectContent>
                {options.drivers.map((d: any) => (
                    <SelectItem key={d.id} value={d.id}>{d.name}</SelectItem>
                ))}
                </SelectContent>
            </Select>
            </div>

            {/* 日時選択 */}
            <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
                <Label>出発日時</Label>
                <Input
                type="datetime-local"
                className="bg-gray-700 border-gray-600 text-white"
                value={departureTime}
                onChange={(e) => setDepartureTime(e.target.value)}
                />
            </div>
            <div className="grid gap-2">
                <Label>到着日時</Label>
                <Input
                type="datetime-local"
                className="bg-gray-700 border-gray-600 text-white"
                value={arrivalTime}
                onChange={(e) => setArrivalTime(e.target.value)}
                />
            </div>
            </div>
        </div>
        )}

        <DialogFooter>
        <Button variant="secondary" onClick={() => setIsOpen(false)}>キャンセル</Button>
        <Button className="bg-blue-600 hover:bg-blue-500" onClick={handleSubmit}>登録する</Button>
        </DialogFooter>
    </DialogContent>
    </Dialog>
);
}
