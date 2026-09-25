from flask import Flask, render_template, request, jsonify
import time

app = Flask(__name__)

@app.route('/')
def home():
    return render_template('index.html')

@app.route('/calculate', methods=['POST'])
def calculate():
    try:
        # รับข้อมูลจาก request
        data = request.get_json()
        input_value = data.get('input', 0)
        
        # ตัวอย่างการคำนวณ: คูณ input ด้วย 2
        result = input_value * 2
        
        # จำลองการหน่วงเวลา (เพื่อให้เห็นว่า endpoint ทำงาน)
        time.sleep(1)
        
        # ส่งผลลัพธ์กลับ
        return jsonify({'result': result})
    except Exception as e:
        return jsonify({'error': str(e)}), 400

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000, debug=True)