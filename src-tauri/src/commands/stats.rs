
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_dashboard_stats(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    // 基本班级信息
    let cohort = sqlx::query_as::<_, (String, String, String)>(
        "SELECT cohort_name, class_name, status FROM cohort WHERE id = ?1"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 学生统计
    let (total_students, male_count, female_count): (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), SUM(CASE WHEN gender = '男' THEN 1 ELSE 0 END), SUM(CASE WHEN gender = '女' THEN 1 ELSE 0 END)
         FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常'"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 今日作业统计（按学生-作业记录维度计算）
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let (hw_count, hw_total_records, hw_completed_records, hw_rate): (i64, i64, i64, f64) = sqlx::query_as(
        "SELECT COUNT(DISTINCT h.id) as hw_count,
            COUNT(hr.id) as total_records,
            COUNT(CASE WHEN hr.status = '已完成' THEN 1 END) as completed_records,
            CASE WHEN COUNT(hr.id) > 0
                THEN CAST(COUNT(CASE WHEN hr.status = '已完成' THEN 1 END) AS REAL) /
                     CAST(COUNT(hr.id) AS REAL)
                ELSE 0.0 END as rate
         FROM homework h
         LEFT JOIN homework_record hr ON hr.homework_id = h.id
         WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL AND h.publish_date = ?2"
    )
    .bind(cohort_id)
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 今日考勤
    let (normal, late, early, leave_, absent): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '迟到' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '早退' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '请假' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '旷课' THEN 1 ELSE 0 END)
         FROM attendance WHERE cohort_id = ?1 AND attendance_date = ?2"
    )
    .bind(cohort_id)
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 待处理作业（未登记的作业数）
    let (pending_hw,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework WHERE cohort_id = ?1 AND deleted_at IS NULL AND publish_date <= ?2
         AND (SELECT COUNT(*) FROM homework_record WHERE homework_id = homework.id AND status != '未登记') <
             (SELECT COUNT(*) FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常')"
    )
    .bind(cohort_id)
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 待处理考勤
    let attendance_today = normal + late + early + leave_ + absent;

    // 重点关注学生
    let focus = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, student_no FROM student
         WHERE cohort_id = ?1 AND deleted_at IS NULL AND is_focus = 1
         ORDER BY student_no ASC LIMIT 10"
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let focus_students: Vec<serde_json::Value> = focus.into_iter().map(|(id, name, no)| {
        serde_json::json!({
            "id": id, "name": name, "student_no": no, "reason": "重点关注"
        })
    }).collect();

    Ok(serde_json::json!({
        "cohort_name": cohort.0,
        "class_name": cohort.1,
        "status": cohort.2,
        "total_students": total_students,
        "male_count": male_count,
        "female_count": female_count,
        "today_homework_count": hw_count,
        "today_homework_total_records": hw_total_records,
        "today_homework_completed": hw_completed_records,
        "today_homework_rate": hw_rate,
        "today_attendance_normal": normal,
        "today_attendance_late": late,
        "today_attendance_early": early,
        "today_attendance_leave": leave_,
        "today_attendance_absent": absent,
        "pending_homework": pending_hw,
        "pending_attendance": attendance_today == 0,
        "focus_students": focus_students
    }))
}

#[tauri::command]
pub async fn homework_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let (total, avg_rate, total_incomplete): (i64, f64, i64) = sqlx::query_as(
        "SELECT COUNT(*) as total,
            COALESCE(AVG(CASE WHEN total_records > 0 THEN CAST(completed AS REAL) / total_records ELSE 0 END), 0) as avg_rate,
            COALESCE(SUM(incomplete), 0) as total_incomplete
         FROM (
             SELECT h.id,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id) as total_records,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status = '已完成') as completed,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status IN ('未登记', '未完成')) as incomplete
             FROM homework h WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL
         )"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 连续未交 (连续3次以上未交)
    let consecutive = sqlx::query_as::<_, (i64, String, String, i64)>(
        "SELECT s.id, s.name, s.student_no, COUNT(*) as c
         FROM student s
         JOIN homework_record hr ON hr.student_id = s.id
         JOIN homework h ON h.id = hr.homework_id AND h.cohort_id = ?1
         WHERE hr.status IN ('未登记', '未完成') AND s.deleted_at IS NULL
         GROUP BY s.id HAVING c >= 3
         ORDER BY c DESC LIMIT 20"
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let consecutive_incomplete: Vec<serde_json::Value> = consecutive.into_iter().map(|(id, name, no, c)| {
        serde_json::json!({ "student_id": id, "student_name": name, "student_no": no, "count": c })
    }).collect();

    Ok(serde_json::json!({
        "total": total,
        "avg_rate": avg_rate,
        "total_incomplete": total_incomplete,
        "consecutive_incomplete": consecutive_incomplete
    }))
}

#[tauri::command]
pub async fn score_statistics_cohort(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let (exams_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (subjects_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM subject"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let records = sqlx::query_as::<_, (String, String, f64, f64, f64)>(
        "SELECT e.name, sub.name,
            AVG(sc.score_value), MAX(sc.score_value), MIN(sc.score_value)
         FROM score sc
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL
         GROUP BY e.id, sub.id
         ORDER BY e.exam_date DESC"
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let records_json: Vec<serde_json::Value> = records.into_iter().map(|(exam, sub, avg, max, min)| {
        serde_json::json!({
            "exam_name": exam, "subject_name": sub, "avg_score": avg, "max_score": max, "min_score": min
        })
    }).collect();

    Ok(serde_json::json!({
        "exams_count": exams_count,
        "subjects_count": subjects_count,
        "records": records_json
    }))
}

#[tauri::command]
pub async fn get_student_profile(
    state: State<'_, AppState>,
    student_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let student = sqlx::query_as::<_, super::student::Student>(
        "SELECT * FROM student WHERE id = ?1"
    )
    .bind(student_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("获取学生信息失败: {}", e))?;

    // 作业统计
    let (hw_total, hw_completed, hw_rate, consecutive): (i64, i64, f64, i64) = sqlx::query_as(
        "SELECT COUNT(*),
            SUM(CASE WHEN status = '已完成' THEN 1 ELSE 0 END),
            CASE WHEN COUNT(*) > 0 THEN CAST(SUM(CASE WHEN status = '已完成' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) ELSE 0 END,
            (SELECT COUNT(*) FROM homework_record hr2
             JOIN homework h2 ON hr2.homework_id = h2.id
             WHERE hr2.student_id = ?1 AND hr2.status IN ('未登记', '未完成')
             AND h2.deleted_at IS NULL
             AND hr2.id >= COALESCE(
                 (SELECT MAX(id) FROM homework_record WHERE student_id = ?1 AND status = '已完成' AND homework_id IN (SELECT id FROM homework WHERE deleted_at IS NULL)),
                 0
             ))
         FROM homework_record hr
         JOIN homework h ON hr.homework_id = h.id
         WHERE hr.student_id = ?1 AND h.deleted_at IS NULL"
    )
    .bind(student_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 考勤统计
    let (att_total, att_normal, att_abnormal, att_rate): (i64, i64, i64, f64) = sqlx::query_as(
        "SELECT COUNT(*),
            SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status != '正常' THEN 1 ELSE 0 END),
            CASE WHEN COUNT(*) > 0 THEN CAST(SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) ELSE 0 END
         FROM attendance WHERE student_id = ?1"
    )
    .bind(student_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 成绩
    let scores = sqlx::query_as::<_, (String, String, Option<f64>)>(
        "SELECT e.name, sub.name, sc.score_value
         FROM score sc
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE sc.student_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL
         ORDER BY e.exam_date DESC LIMIT 50"
    )
    .bind(student_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let scores_json: Vec<serde_json::Value> = scores.into_iter().map(|(exam, sub, score)| {
        serde_json::json!({ "exam_name": exam, "subject_name": sub, "score_value": score })
    }).collect();

    // 奖惩记录
    let behaviors = sqlx::query_as::<_, super::affair::BehaviorRecord>(
        "SELECT * FROM behavior_record WHERE student_id = ?1 ORDER BY record_date DESC LIMIT 50"
    )
    .bind(student_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "student": student,
        "homework": {
            "total": hw_total,
            "completed": hw_completed,
            "rate": hw_rate,
            "consecutive_incomplete": consecutive
        },
        "attendance": {
            "total": att_total,
            "normal": att_normal,
            "abnormal": att_abnormal,
            "rate": att_rate
        },
        "scores": scores_json,
        "behaviors": behaviors
    }))
}
