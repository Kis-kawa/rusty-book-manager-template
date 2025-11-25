import { Button } from "@/components/ui/button";
import Link from "next/link";


export default function Home() {
  return (
    <div className="flex flex-col md:flex-row w-full h-screen items-center justify-center bg-gray-50">

      {/* 左 bg-red-400  */}
      <div className="flex flex-col justify-center items-center h-[50vh] w-full md:w-2/5">
        <img src="/images/rogo_icon.png" className="w-100"/>
        <span className="text-gray-600">Ⓒ岸川観光バス会社</span>
      </div>

      {/* 右 bg-blue-100 */}
      <div className="flex flex-col justify-center p-10 h-[50vh] w-full md:w-3/5">
        <h1 className="text-4xl font-bold text-black-600">産技往復便</h1>
        <p className="mt-4 text-gray-500">予約ホームページ</p>
        <div className="flex gap-4 mt-8 flex-wrap">
          <Button asChild>
            <Link href="/login">ログイン</Link>
          </Button>
          <Button variant="outline">
            <Link href="/signup">サインアップ</Link>
          </Button>
        </div>
      </div>
    </div>
  );
}
