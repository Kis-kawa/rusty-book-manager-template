"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import Link from "next/link";

// Rustから送られてくるJSONで形定義
type Trip = {
  trip_id: string;
  source: string;
  destination: string;
  departure_time: string;
  arrival_time: string;
  vehicle_name: string;
  status: string;
};

// 日付をきれいに表示するための関数
const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleString('ja-JP', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    weekday: 'short',
  });
};

// 予約ボタンを押した時の処理
  const handleReserve = async (tripId: string) => {
    // ログインチェック
    const savedUser = localStorage.getItem("currentUser");
    if (!savedUser) {
      alert("予約するにはログインしてください");
      return;
    }
    const user = JSON.parse(savedUser);

    // 確認ダイアログ
    if (!confirm("この便を予約しますか？")) return;

    try {
      const res = await fetch("http://localhost:8000/reservations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          trip_id: tripId,
          user_id: user.user_id,
        }),
      });

      if (res.ok) {
        const text = await res.text();
        alert(text);
      } else if (res.status === 422) {
        alert("満席のため予約できませんでした");
      } else if (res.status === 409) {
        alert("すでにこの便を予約済みです");
      } else {
        alert("予約に失敗しました");
      }
    } catch (error) {
      console.error(error);
      alert("エラーが発生しました");
    }
  };


export default function Home() {
  const [userName, setUserName] = useState<string | null>(null);
  const [trips, setTrips] = useState<Trip[]>([]); // 運行便のリスト
  const [isLoading, setIsLoading] = useState(true);
  const [isMaintenance, setIsMaintenance] = useState<boolean | null>(null);
  const [showPast, setShowPast] = useState(false); // 過去便の表示スイッチ

  useEffect(() => {
    fetch("http://localhost:8000/admin/maintenance")
      .then((res) => res.json())
      .then((data) => setIsMaintenance(data))
      .catch((e) => {
        console.error("Status check failed", e);
        // 通信エラー時はとりあえず通常表示にするか、エラー表示にするか
        // ここでは通常表示(false)にしておく
        setIsMaintenance(false);
      });
  }, []);


	// 画面が表示されたら
  useEffect(() => {
    // ログインユーザーの確認
    const savedUser = localStorage.getItem("currentUser");
    if (savedUser) {
      const user = JSON.parse(savedUser);
      setUserName(user.name);
    }

    // バス便データの取得
    const fetchTrips = async () => {
      try {
        const res = await fetch("http://localhost:8000/trips");
        if (res.ok) {
          const data = await res.json();
          setTrips(data);
        } else {
          console.error("データの取得に失敗しました");
        }
      } catch (error) {
        console.error("サーバーに接続できません", error);
      } finally {
        setIsLoading(false);
      }
    };

    fetchTrips();
  }, []);

  if (isMaintenance) {
    return (
      <div className="min-h-screen bg-gray-950 flex flex-col items-center justify-center text-white p-4">
        <div className="bg-gray-900 border border-red-800 p-8 rounded-xl shadow-2xl max-w-md w-full text-center space-y-6">
          <div className="flex justify-center">
            <div className="bg-red-900/30 p-4 rounded-full">
              <h1 className="text-3xl font-bold text-red-500">
                メンテナンス中！！！
              </h1>
            </div>
          </div>

          <h1 className="text-3xl font-bold text-red-500">
            ただいまメンテナンス中です
          </h1>

          <p className="text-gray-300 leading-relaxed">
            現在、システムの点検・改修を行っております。<br />
            ご不便をおかけしますが、再開までしばらくお待ちください。
          </p>

          <div className="pt-4 text-sm text-gray-500">
            System Maintenance Mode
          </div>
        </div>
      </div>
    );
  }

  const now = new Date();

  const visibleTrips = trips.filter((trip) => {
    // スイッチがONなら全部見せる
    if (showPast) return true;

    // スイッチOFFなら、到着時刻が「未来」のものだけ残す
    return new Date(trip.arrival_time) > now;
  });
return (
    // 全体を縦並びのフレックスボックスに、min-h-screenにしてスクロール可能に
    <div className="flex flex-col min-h-screen w-full bg-gray-50">

      {/* --- 上部エリア（ロゴとタイトル・ボタン） --- */}
      <div className="flex flex-col md:flex-row w-full h-auto md:h-[50vh] items-center justify-center gap-12 py-10 md:py-0">

        {/* 左: ロゴエリア */}
        <div className="flex flex-col items-center">
          <img src="/images/rogo_icon.png" className="w-64 md:w-80 object-contain"/>
          <span className="text-gray-600 mt-2 text-sm">Ⓒ岸川観光バス会社</span>
        </div>

        {/* 右: タイトル・ボタンエリア */}
        <div className="flex flex-col items-center md:items-start text-center md:text-left">
          <h1 className="text-4xl font-bold text-black-600">産技往復便</h1>
          <p className="mt-4 text-gray-500 mb-6">予約ホームページ</p>

          <div className="flex gap-4 items-center">
            {userName ? (
              <>
                <span className="font-bold">{userName} さん</span>
                <Button variant="secondary" asChild className="mr-2">
                  <Link href="/mypage">予約確認</Link>
                </Button>
                <Button variant="outline" onClick={() => {
                  localStorage.removeItem("currentUser");
                  window.location.reload();
                }}>ログアウト</Button>
              </>
            ) : (
                <>
                  <Button asChild>
                    <Link href="/login">ログイン</Link>
                  </Button>
                  <Button variant="outline">
                    <Link href="/signup">サインアップ</Link>
                  </Button>
                </>
            )}
          </div>
        </div>
      </div>

      {/* --- 下部エリア（運行スケジュール） --- */}
      <div className="w-full p-10">
        <h2 className="text-2xl font-bold mb-6">運行スケジュール</h2>

        {isLoading ? (
          <p>読み込み中...</p>
        ) : (
          <div className="max-w-6xl mx-auto space-y-6">

            {/* ★表示切り替えスイッチエリア */}
            <div className="flex justify-end items-center gap-2 mb-4">
              <input
                type="checkbox"
                id="showPast"
                checked={showPast}
                onChange={(e) => setShowPast(e.target.checked)}
                className="w-4 h-4 cursor-pointer"
              />
              <label htmlFor="showPast" className="cursor-pointer text-gray-400 hover:text-white select-none">
                終了した便も表示する
              </label>
            </div>

            {/* ★リスト表示エリア */}
            {visibleTrips.length === 0 ? (
              <p className="text-gray-500 text-center py-8">
                表示できる運行便がありません
              </p>
            ) : (
              // ここで元のグリッドレイアウト(grid-cols-3)を使います
              // ループさせるのは trips ではなく visibleTrips です
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {visibleTrips.map((trip) => (
                  <Card key={trip.trip_id} className="hover:shadow-lg transition-shadow border-l-4 border-l-blue-500">
                    <CardHeader className="pb-2">
                      <div className="flex justify-between items-start">
                        <div className="text-sm font-bold text-blue-600 bg-blue-50 px-2 py-1 rounded">
                          {trip.vehicle_name}
                        </div>
                        {/* statusがある場合のみ表示 (scheduledなど) */}
                        {trip.status === "scheduled" && (
                          <span className="text-xs text-green-600 border border-green-200 px-2 py-1 rounded-full bg-green-50">
                            運行予定
                          </span>
                        )}
                      </div>
                      <CardTitle className="text-xl mt-2 flex items-center gap-2">
                        {trip.source}
                        <span className="text-gray-400">→</span>
                        {trip.destination}
                      </CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="flex flex-col gap-2 mt-2">
                        <div className="flex justify-between items-center border-b pb-2">
                          <span className="text-gray-500 text-sm">出発</span>
                          {/* formatDate関数がある前提です */}
                          <span className="font-bold text-lg">{new Date(trip.departure_time).toLocaleString()}</span>
                        </div>
                        <div className="flex justify-between items-center">
                          <span className="text-gray-500 text-sm">到着</span>
                          <span className="font-bold ">{new Date(trip.arrival_time).toLocaleString()}</span>
                        </div>

                        <Button
                          className="w-full mt-4 bg-blue-600 hover:bg-blue-700"
                          onClick={() => handleReserve(trip.trip_id)} // 予約処理(関数名は適宜合わせてください)
                        >
                          予約する
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
