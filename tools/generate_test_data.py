"""
This Python script deterministically generates sample folders and files 
for testing the nfd2nfc program, always producing consistent results. 
If the target folder already exists, it will be deleted and recreated. 
Note that most names behave differently under NFD and NFC normalization, 
though some English names are also included.
"""

import os
import random
import shutil

TOTAL_ITEMS = 10000
MAX_DEPTH = 4
FOLDER_RATIO = 0.4
ROOT_FOLDER = os.path.join(os.getcwd(), "test_data")
RANDOM_SEED = 42

NAMES_LIST = ["Café", "naïve", "résumé", "한글폴더", "été", "coöperate", "São_Paulo", "niño", "Ångström", "Smörgåsbord", "DéjàVu", "über", "jalapeño", "CrèmeBrûlée", "Köln", "München", "Zürich", "Tokyo", "NewYork", "Sydney", "fiancée", "Noël", "vis-à-vis", "façade", "garçon", "mañana", "Tōkyō", "Håndverk", "Bjørn", "Jürgen", "Göteborg", "Espèce", "Málaga", "Genève", "Çalışkan", "İstanbul", "Vārāṇasī", "L’Université", "México", "Hawaiʻi", "Zürichberg", "Cambridge", "London", "Berlin", "Paris", "Chicago", "Seoul", "Busan", "Madrid", "Rome", "Amsterdam", "Oslo", "Reykjavík", "Stavanger", "Tromsø", "Helsinki", "Warszawa", "Praha", "Bratislava", "Budapest", "București", "Sofia", "Beograd", "Skopje", "Tirana", "Riga", "Vilnius", "Tallinn", "Kyiv", "Minsk", "Astana", "Tashkent", "Ulaanbaatar", "한글파일", "한국어이름", "테스트폴더", "자료보관", "프로젝트폴더", "문서저장소", "연습파일", "데이터셋", "샘플데이터", "보고서", "Malmö", "Bordeaux", "Strasbourg", "Frankfurt", "Wien", "Brno", "Graz", "Ljubljana", "Dubrovnik", "Sarajevo", "Podgorica", "Valletta", "Bern", "La Paz", "Caracas", "Santiago", "BuenosAires", "Montevideo", "Brasília", "Asunción", "Sucre", "Quito", "Lima", "Bogotá", "Havana", "논문자료", "참고문헌", "실험데이터", "연구결과", "데이터분석", "프로그램코드", "샘플스크립트", "텍스트파일", "사용자설정", "환경설정", "테마설정", "시스템로그", "웹서버로그", "앱설정파일", "개발노트", "디자인초안", "모형설계", "프로젝트기획", "UIUX디자인", "코드리뷰", "마케팅전략", "제품설명서", "계약서", "보고서초안", "디자인시안", "일정관리", "출근기록", "연구일지", "장바구니", "북마크", "다운로드파일", "브라우저설정", "게임저장파일", "영상편집파일", "사진백업", "음원파일", "문서백업", "이메일첨부", "클라우드업로드", "로컬캐시", "백업폴더", "버전관리", "웹사이트스크랩", "기사저장", "뉴스클리핑", "회의기록", "직원명단", "고객데이터", "CRM정보", "피드백정리", "사용후기", "제품리뷰", "구매내역", "거래기록", "영수증보관", "세금신고", "원천징수", "급여명세서", "인사기록", "조직도", "공지사항", "업무분장", "사내메신저로그", "개발문서", "소스코드", "기술사양", "API문서", "함수정의", "인터페이스설계", "백엔드로그", "프론트엔드파일", "데이터베이스백업", "서버설정", "네트워크설정", "보안정책", "비밀번호관리", "사용자권한", "시스템로그", "이벤트기록", "운영체제설정", "모바일앱데이터", "앱스토어리뷰", "패치노트", "버전업데이트", "시스템패치", "소프트웨어업데이트", "오류리포트", "버그수정내역", "개선점정리", "고객문의", "사용설명서", "튜토리얼파일", "FAQ문서", "지원센터", "기술지원기록", "제품가이드", "트러블슈팅가이드", "문제해결방법", "IT지원", "네트워크로그", "서버백업", "코드디버깅", "테스트결과", "성능테스트", "로드테스트", "데이터마이그레이션", "마이그레이션로그", "프로젝트타임라인", "일정조정", "개발스프린트", "업무보고", "회의록", "작업진행상황", "제품개발일지", "협업문서", "디자인프로토타입", "인터랙션설계", "사용성테스트", "사용자리서치", "고객분석", "트렌드리포트", "마케팅분석", "경쟁사분석", "사업계획서", "예산계획", "재무보고서", "연간보고서", "투자계획", "주주총회기록", "법률문서", "계약서사본", "소송기록", "법률자문", "감사보고서", "품질관리", "생산기록", "유통기록", "물류데이터", "창고관리", "재고기록", "공급망관리", "구매주문서", "송장", "출고기록", "배송추적", "반품기록", "고객클레임", "사용자로그", "앱사용패턴", "데이터수집로그", "로그분석", "통계파일", "데이터시각화", "대시보드데이터", "보고서템플릿", "발표자료", "슬라이드파일", "강의자료", "학습가이드", "훈련계획", "연수자료", "자기계발노트", "다이어리", "일정표", "독서기록", "필기노트", "회고일지", "심리테스트기록", "건강관리", "운동계획", "영양정보", "식단기록", "재무목표", "예산관리", "가계부", "투자기록", "포트폴리오", "증권거래내역", "암호화폐거래내역", "부동산계약", "임대계약", "자동차등록", "보험서류", "병원기록", "진료내역", "의료보험", "백신접종기록", "여행계획", "항공권예매", "호텔예약", "관광명소리스트", "짐싸기리스트", "여행사진", "여행일기", "여행후기", "자동차정비기록", "수리내역", "DIY프로젝트", "취미기록", "악보", "음악작업파일", "영상제작", "애니메이션작업", "게임기획서", "보드게임규칙", "스포츠기록", "경기분석", "팀전술", "전술노트", "훈련계획", "코칭자료", "선수데이터", "팬클럽기록", "커뮤니티운영기록", "동호회활동", "기부내역", "봉사활동기록", "종교활동", "명상일지", "영성노트", "가문족보", "가족사진정리", "가계도", "유산계획", "자서전작성", "개인프로젝트", "창작노트", "소설초안", "시나리오", "드라마스크립트", "연극대본", "만화스토리보드", "웹툰기획", "유튜브채널운영", "콘텐츠기획", "블로그초안", "뉴스레터기획", "광고캠페인", "브랜드전략", "소셜미디어분석", "인플루언서데이터", "광고성과분석", "검색엔진최적화", "키워드분석", "도메인기록", "웹사이트디자인", "사용자경험분석", "AB테스트기록", "웹접근성테스트", "온라인쇼핑기록", "리뷰데이터", "전자상거래기록", "결제이력", "보안로그", "암호관리", "이중인증기록", "개인정보보호", "데이터암호화", "디지털유산", "온라인계약", "전자서명", "클라우드스토리지", "파일공유로그", "원격근무기록", "협업툴데이터", "원격회의기록", "화상회의로그", "온라인세미나", "코딩챌린지기록", "해커톤기록", "오픈소스기여기록", "깃허브리포지토리", "코드커밋로그", "버전컨트롤기록", "소프트웨어배포기록", "운영자동화스크립트", "서버모니터링로그", "클라우드리소스관리", "컨테이너배포로그", "DevOps워크플로우", "CICD파이프라인", "AI모델학습로그", "머신러닝데이터셋", "데이터라벨링기록", "자연어처리모델", "이미지처리모델", "음성인식데이터", "추천시스템기록", "빅데이터분석", "IoT디바이스로그", "스마트홈기록", "자율주행기록", "드론비행기록", "위성데이터", "기상분석", "환경모니터링", "탄소배출기록", "신재생에너지기록", "에너지소비기록", "도시계획데이터", "교통패턴분석", "스마트시티프로젝트", "공공데이터활용", "정부정책기록", "법안분석", "선거데이터", "경제지표", "사회조사기록", "글로벌트렌드분석", "국제관계기록", "ガラス", "ｶﾞﾗｽ", "ガラス", "ﾊﾟｿｺﾝ", "パソコン", "ｽﾀｰﾄ", "スタート", "組み合わせ", "合字", "𩸽", "魚𩸽", "ﾃｽﾄ", "テスト", "ﾃﾞｨｽﾌﾟﾚｲ", "ディスプレイ", "ﾏｲｸ", "マイク", "ﾗｲﾄ", "ライト", "ｻｰﾋﾞｽ", "サービス", "ｽｰﾊﾟｰ", "スーパー", "ｺﾝﾋﾞﾆ", "コンビニ", "ｺｰﾋｰ", "コーヒー", "ｼｬﾝﾌﾟｰ", "シャンプー", "ﾊﾟｽﾀ", "パスタ", "ﾊﾞｲｵﾘﾝ", "バイオリン", "ﾋﾞｰﾙ", "ビール", "ﾃﾚﾋﾞ", "テレビ", "ﾌﾞﾗｳｻﾞ", "ブラウザ", "ﾌｧｲﾙ", "ファイル", "ﾀｲﾄﾙ", "タイトル", "ﾌﾟﾛｼﾞｪｸﾄ", "プロジェクト", "ﾊﾟｰﾂ", "パーツ", "ﾌﾟﾛｸﾞﾗﾑ", "プログラム", "ﾒﾆｭｰ", "メニュー", "ﾃﾞｨｽｸ", "ディスク", "ﾌｫﾙﾀﾞ", "フォルダ", "ﾚﾎﾟｰﾄ", "レポート", "ﾃﾞｰﾀ", "データ", "ﾋﾟｱﾉ", "ピアノ", "ｼｽﾃﾑ", "システム", "ﾄﾞｷｭﾒﾝﾄ", "ドキュメント", "ﾌﾟﾘﾝﾀ", "プリンタ", "ﾌｨﾙﾀｰ", "フィルター", "ﾃｷｽﾄ", "テキスト", "ﾏﾆｭｱﾙ", "マニュアル", "ﾊﾞｯｸｱｯﾌﾟ", "バックアップ", "ﾊﾞｯﾃﾘｰ", "バッテリー", "ﾈｯﾄﾜｰｸ", "ネットワーク", "ﾓﾆﾀｰ", "モニター", "ﾏｳｽ", "マウス", "ﾍｯﾄﾞﾌｫﾝ", "ヘッドフォン", "ﾃﾞｼﾞﾀﾙ", "デジタル", "ﾒﾓﾘ", "メモリ", "ｹｰﾌﾞﾙ", "ケーブル", "ﾌﾞﾙｰﾚｲ", "ブルーレイ", "ﾊｰﾄﾞﾃﾞｨｽｸ", "ハードディスク", "ﾃﾞﾌｫﾙﾄ", "デフォルト", "ｺﾝﾄﾛｰﾙ", "コントロール", "ﾌﾞﾗｳｻﾞ", "ブラウザ", "ﾎﾟｲﾝﾀ", "ポインタ", "ﾃｷｽﾄ", "テキスト", "ﾌｫｰﾏｯﾄ", "フォーマット", "ﾊﾟｽﾜｰﾄﾞ", "パスワード", "ﾀｲﾌﾟ", "タイプ", "ﾒﾓ", "メモ", "ﾃｽﾄﾌｧｲﾙ", "テストファイル", "ｵｰﾃﾞｨｵ", "オーディオ", "ｽﾋﾟｰｶｰ", "スピーカー", "ﾄﾗﾌﾞﾙ", "トラブル", "ﾌﾟﾛﾀﾞｸﾄ", "プロダクト", "ｹﾞｰﾑ", "ゲーム", "ﾌﾟﾛﾌｨｰﾙ", "プロフィール", "ﾃﾞｨｽｶｳﾝﾄ", "ディスカウント", "ﾌｨｰﾄﾞﾊﾞｯｸ", "フィードバック", "ｻﾎﾟｰﾄ", "サポート", "ﾏﾆｭｱﾙﾓｰﾄﾞ", "マニュアルモード", "ﾕｰｻﾞｰ", "ユーザー", "ﾄﾞﾗｲﾊﾞ", "ドライバ", "ﾊｰﾄﾞｳｪｱ", "ハードウェア", "ﾌｧｰﾑｳｪｱ", "ファームウェア", "ﾃﾞｨﾚｸﾄﾘ", "ディレクトリ", "ﾚｼﾞｽﾄﾘ", "レジストリ", "ﾄﾞｷｭﾒﾝﾀﾘｰ", "ドキュメンタリー", "ﾄﾗﾝｽﾌｧｰ", "トランスファー", "ﾃﾞｨｽﾄﾘﾋﾞｭｰｼｮﾝ", "ディストリビューション", "ｵﾌﾟｼｮﾝ", "オプション", "ﾃﾞﾌｫﾙﾄﾓｰﾄﾞ", "デフォルトモード", "ﾛｸﾞｲﾝ", "ログイン", "ﾛｸﾞｱｳﾄ", "ログアウト", "ﾊﾟﾗﾒｰﾀ", "パラメータ", "ﾄﾗｯｸ", "トラック", "ﾀﾞｳﾝﾛｰﾄﾞ", "ダウンロード", "ﾃﾞｰﾀﾍﾞｰｽ", "データベース", "ｱｯﾌﾟﾃﾞｰﾄ", "アップデート", "ﾀﾞｲﾅﾐｯｸ", "ダイナミック", "ﾃﾝﾌﾟﾚｰﾄ", "テンプレート", "ﾀﾞｯｼｭﾎﾞｰﾄﾞ", "ダッシュボード", "ﾊｲﾗｲﾄ", "ハイライト", "ﾏｰｹｯﾄ", "マーケット", "ﾌﾟﾛﾌｪｯｼｮﾅﾙ", "プロフェッショナル", "ﾋﾟｸｾﾙ", "ピクセル", "ﾊﾟｰｽﾍﾟｸﾃｨﾌﾞ", "パースペクティブ", "ﾀﾞｲﾚｸﾄ", "ダイレクト", "ﾎﾞﾘｭｰﾑ", "ボリューム", "ﾋﾞｼﾞｭｱﾙ", "ビジュアル", "ﾀﾞﾌﾞﾙｸﾘｯｸ", "ダブルクリック", "ﾃﾞｨﾚｸｼｮﾝ", "ディレクション", "ﾃﾞｻﾞｲﾝ", "デザイン", "ﾊﾞﾗﾝｽ", "バランス", "ﾀﾞﾐｰ", "ダミー", "ﾃｨｰﾑ", "チーム", "ﾋﾟﾗﾐｯﾄﾞ", "ピラミッド", "ﾌｧｸﾄ", "ファクト", "ﾃﾞｰﾀｾｯﾄ", "データセット", "Café", "naïve", "fiancée", "résumé", "Noël", "vis-à-vis", "façade", "garçon", "été", "CrèmeBrûlée", "über", "Köln", "München", "Zürich", "Málaga", "Genève", "Espèce", "jalapeño", "coöperate", "São_Paulo", "México", "fiancée", "déjà_vu", "maître", "éclair", "crème", "coïncidence", "élève", "étoile", "être", "façade", "façade_vieille", "hôpital", "île", "réservoir", "sûr", "téléphone", "vérité", "vêtu", "barrière", "brûler", "caractère", "château", "clarté", "collègue", "conférence", "défense", "début", "désespoir", "diplômé", "éblouir", "économie", "écœurant", "édifice", "église", "élégance", "élément", "éloquence", "émission", "émotion", "enchaînement", "énergie", "énigme", "épanouissement", "épicerie", "épisode", "époque", "équipe", "équité", "érosion", "erreur", "escalier", "esthétique", "étouffant", "évanescent", "évidence", "évolution", "éxagération", "exécution", "exaltation", "exclamation", "expansion", "expérience", "explication", "explosion", "expression", "extension", "facilité", "façonnage", "famille", "fatalité", "fébrilité", "félicité", "féminité", "fiancé", "fidélité", "finance", "floraison", "formation", "formalité", "franchise", "fréquence", "galerie", "génération", "générosité", "gloire", "grandeur", "harmonie", "héroïsme", "hospitalité", "humanité", "hymne", "identité", "ignorance", "imagination", "immersion", "implication", "impossibilité", "indépendance", "indifférence", "indulgence", "industrie", "infini", "influence", "innovation", "instruction", "intégrité", "intelligence", "intensité", "interaction", "interférence", "introduction", "intuition", "invention", "invitation", "jeunesse", "journalisme", "légitimité", "liberté", "littérature", "logistique", "luminosité", "machine", "magnanimité", "majesté", "maîtrise", "maternité", "maturité", "mémoire", "métaphore", "modération", "modernité", "moralité", "motivation", "mouvement", "nécessité", "noblesse", "nuance", "objectif", "obligation", "observation", "occasion", "occupation", "opacité", "opération", "opportunité", "opposition", "optimisme", "organisation", "originalité", "orthodoxie", "panthéon", "parallèle", "patience", "perception", "perfection", "performance", "persévérance", "persuasion", "philosophie", "photographie", "popularité", "précision", "préjudice", "préoccupation", "prérogative", "présence", "préservation", "prévention", "primauté", "priorité", "problématique", "procédure", "proclamation", "production", "professionnalisme", "programmation", "progression", "prohibition", "prolongation", "promesse", "propriété", "prospérité", "protection", "protestation", "proximité", "psychisme", "publicité", "purification", "qualité", "quotidien", "rationalité", "réalité", "rébellion", "reconnaissance", "recommandation", "réflexion", "réforme", "régularité", "relation", "relativité", "réminiscence", "responsabilité", "restructuration", "révélation", "révolution", "satisfaction", "sentimentalité", "sérénité", "signification", "similitude", "solidarité", "solution", "sophistication", "spécificité", "spiritualité", "spontanéité", "stabilité", "stimulation", "stratégie", "structure", "subjectivité", "subtilité", "suggestion", "supériorité", "surveillance", "surprise", "symbiose", "synthèse", "télécommunication", "témérité", "témoin", "tendresse", "tolérance", "totalité", "tradition", "transformation", "transition", "transmission", "transparence", "turbulence", "unicité", "urbanisation", "utilité", "variation", "véhémence", "véracité", "vigilance", "virilité", "visibilité", "vitalité", "vocation", "volatilité", "volonté", "vulnérabilité", "bài_hát", "cà_phê", "cẩm_nang", "cơ_hội", "giấc_mơ", "lặng_lẽ", "lý_tưởng", "mưa_giông", "nhiệt_đới", "quốc_gia", "sáng_tạo", "thông_tin", "truyền_thống", "văn_hóa", "vũ_trụ", "yêu_thương", "đồng_hành", "hội_thảo", "thảo_luận", "hệ_thống", "ánh_sáng", "bài_tập", "bão_táp", "biểu_tượng", "bộ_sưu_tập", "chăm_chỉ", "chữ_viết", "đại_dương", "đơn_giản", "gia_đình", "giai_điệu", "hội_nghị", "khu_vực", "khám_phá", "kết_nối", "khả_năng", "không_gian", "lãng_mạn", "lập_trình", "máy_móc", "môi_trường", "nghiên_cứu", "nhiệm_vụ", "phát_triển", "phương_pháp", "quan_niệm", "quảng_cáo", "sách_báo", "sáng_kiến", "siêu_thị", "sinh_học", "sự_nghiệp", "tác_phẩm", "tài_nguyên", "tài_năng", "thích_nghi", "thiên_nhiên", "thương_hiệu", "tính_cách", "tập_trung", "thực_phẩm", "thực_vật", "thư_viện", "thời_gian", "tin_tức", "tổ_chức", "tổng_quan", "tương_lai", "vĩnh_cửu", "vinh_quang", "vũ_khí", "vật_liệu", "Академия", "Благодарность", "Великолепие", "Гарантия", "Доверие", "Единство", "Жизнь", "Зарядка", "Изобилие", "Искренность", "Качество", "Лояльность", "Мотивация", "Надежда", "Образование", "Признание", "Реализация", "Свобода", "Сияние", "Уникальность", "Фантазия", "Харизма", "Ценность", "Чувство", "Шедевр", "Щедрость", "Энергия", "Юмор", "Яркость", "Αγάπη", "Βιβλίο", "Γέφυρα", "Δάσκαλος", "Ελευθερία", "Ζωή", "Ηθική", "Θάλασσα", "Ιστορία", "Καλοσύνη", "Λογοτεχνία", "Μουσική", "Νερό", "Οικογένεια", "Παιδεία", "Ρυθμός", "Σοφία", "Ταχύτητα", "Υγεία", "Φως", "Χαρά", "Ψυχή", "apple", "banana", "orange", "strawberry", "blueberry", "raspberry", "blackberry", "lemon", "lime", "mango", "pineapple", "watermelon", "peach", "apricot", "cherry", "coconut", "date", "fig", "grape", "kiwi", "plum", "pear", "cantaloupe", "papaya", "tangerine", "cucumber", "tomato", "avocado", "broccoli", "spinach", "lettuce", "cabbage", "carrot", "potato", "onion", "garlic", "mushroom", "peanut", "almond", "walnut", "cashew", "pecan", "sunflower", "sesame", "pumpkin", "squash", "zucchini", "eggplant", "asparagus", "kale", "celery", "radish", "parsley", "cilantro", "rosemary", "thyme", "basil", "oregano", "sage", "dill", "mint", "chive", "ginger", "turmeric", "cinnamon", "nutmeg", "vanilla", "chocolate", "coffee", "tea", "honey", "sugar", "milk", "butter", "cheese", "yogurt", "bread", "toast", "sandwich", "burger"]

# Set a fixed random seed to ensure deterministic output.
random.seed(RANDOM_SEED)

# If the ROOT_FOLDER already exists, delete it and recreate.
if os.path.exists(ROOT_FOLDER):
    shutil.rmtree(ROOT_FOLDER)
os.makedirs(ROOT_FOLDER, exist_ok=True)

num_folders = int(TOTAL_ITEMS * FOLDER_RATIO)
num_files = TOTAL_ITEMS - num_folders

# Maintain a list of folders with their depth: (folder_path, depth)
folder_list = [(ROOT_FOLDER, 0)]

def get_unique_name(directory, base_name):
    name = base_name
    counter = 1
    
    while True:
        path = os.path.join(directory, name)
        if not os.path.exists(path):
            return name
        name = f"{base_name} ({counter})"
        counter += 1

# Create folders.
created_folders = 0
while created_folders < num_folders:
    valid_folders = [f for f in folder_list if f[1] < MAX_DEPTH - 1]

    parent_folder, depth = random.choice(valid_folders)
    folder_name = get_unique_name(parent_folder, random.choice(NAMES_LIST))
    new_folder = os.path.join(parent_folder, folder_name)
    
    try:
        os.makedirs(new_folder, exist_ok=True)
        folder_list.append((new_folder, depth + 1))
        created_folders += 1
    except Exception as e:
        print(f"Failed to create folder: {e}")

# Create files.
created_files = 0
while created_files < num_files:
    target_folder, _ = random.choice(folder_list)
    file_name = get_unique_name(target_folder, random.choice(NAMES_LIST))
    file_path = os.path.join(target_folder, file_name)

    try:
        with open(file_path, 'w') as f:
            f.write("")  # Create an empty file.
        created_files += 1
    except Exception as e:
        print(f"Failed to create file: {e}")

print(f"Successfully created {num_folders} folders and {num_files} files!")