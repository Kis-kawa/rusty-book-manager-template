"use client"

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input  } from "@/components/ui/input";
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


export default function SingUpPage(){

    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");

    const handleSignUp = async () => {
        try{
        }catch(e){
            console.error(e);
        }
    };

    return (
        <div className="flex h-screen items-center justify-center bg-gray-100">
            <Card className="w-[400px]">
            <CardHeader>
                <CardTitle >産技往復便 サインアップ</CardTitle>
                <CardDescription>産業技術高等専門学校 品川キャンパスと荒川キャンパスをつなぐ往復便の予約システムにサインアップ</CardDescription>
                <CardAction>
                    <Link href="/login">Login</Link>
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
                <Button className="w-full" onClick={handleSignUp}> Sign Up </Button>
            </CardFooter>
            </Card>
        </div>
    );
}
