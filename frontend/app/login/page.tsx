"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input  } from "@/components/ui/input";
import { useRouter } from "next/navigation";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "@/components/ui/card"
import Link from "next/link";

export default function LoginPage(){
    // [変数名, 値を変える関数（用意されてる）] = useState(初期値)
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const router = useRouter();

    const handleLogin = async () => {
        try {
        const response = await fetch("http://localhost:8000/login", {
            method: "POST",
            headers: {
            "Content-Type": "application/json",
            },
            body: JSON.stringify({ email: email, password: password }),
        });

        if (response.ok) {
            const data = await response.json();

            // ブラウザの保存領域(localStorage)にユーザー情報をしまっておく
            localStorage.setItem("currentUser", JSON.stringify(data));
            alert(`ログインしました！ようこそ ${data.name} さん`);

            // トップページ（予約画面）へ移動
            router.push("/");
        } else {
            alert("メールアドレスかパスワードが間違っています");
            }
        } catch (error) {
        console.error(error);
        alert("サーバーに接続できません");
        }
    };

    return(
        <div className="flex h-screen items-center justify-center bg-gray-100">
            <Card className="w-[400px]">
            <CardHeader>
                <CardTitle >産技往復便 ログイン</CardTitle>
                <CardDescription>産業技術高等専門学校 品川キャンパスと荒川キャンパスをつなぐ往復便の予約システムにログイン</CardDescription>
                <CardAction>
                    <Link href="/signup">Sing Up</Link>
                </CardAction>
            </CardHeader>
            <CardContent>
                <div className="flex flex-col gap-5">
                    <div className="flex flex-col gap-1.5">
                        <p className="text-xs text-gray-500">メールアドレス　例）m11111@g.metro-cit.ac.jp</p>
                        <Input
                            type="email"
                            placeholder="メールアドレス"
                            value={email} // 箱の中身を表示
                            onChange={(e) => setEmail(e.target.value)}
                            required
                        />
                    </div>
                    <div className="flex flex-col gap-1.5">
                        <p className="text-xs text-gray-500">パスワード　※特殊文字なし、6~32文字</p>
                        <Input
                            type="password"
                            placeholder="パスワード"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            required
                        />
                    </div>
                </div>
            </CardContent>
            <CardFooter className="flex-col gap-1">
                <Button className="w-full" onClick={handleLogin}> Login </Button>
            </CardFooter>
            </Card>
        </div>
    );
}
