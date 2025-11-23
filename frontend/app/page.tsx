import { Button } from "@/components/ui/button";
import Link from "next/link";


export default function Home() {
  return (
    <div className="p-12">
      <h1 className="text-4xl font-bold text-blue-600">産技往復便</h1>
      <p className="mt-4 text-gray-500">予約ホームページ</p>
      <div className="flex gap-4">
        <Button>
          普通のボタン
        </Button>

        <Button variant="secondary">
          サブボタン
        </Button>

        <Button variant="destructive">
          削除ボタン
        </Button>

        <Button variant="outline">
          枠線ボタン
        </Button>

        <Button asChild>
          <Link href="/login">ログイン画面へ</Link>
        </Button>
      </div>

    </div>
  );
}
